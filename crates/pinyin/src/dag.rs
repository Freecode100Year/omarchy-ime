use crate::cost::weighted_osa;
use crate::PinyinConfig;
use std::{collections::HashSet, sync::OnceLock};

#[derive(Clone, Debug, PartialEq)]
pub struct SyllableEdge {
    pub start: usize,
    pub end: usize,
    pub syllable: String,
    pub cost: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyllableDag {
    pub input: String,
    pub original: String,
    pub edges: Vec<Vec<SyllableEdge>>,
    pub english: Vec<bool>,
    pub config: PinyinConfig,
}

impl SyllableDag {
    pub fn from_input(input: &str, config: &PinyinConfig) -> Self {
        let s = input.to_ascii_lowercase();
        let bytes = s.as_bytes();
        let n = bytes.len();
        let mut edges = vec![Vec::new(); n + 1];
        let mut english = vec![false; n];
        let inventory = syllables_by_length();
        let exact_inventory = syllable_set();
        for start in 0..n {
            if !bytes[start].is_ascii_lowercase() {
                continue;
            }
            for end in start + 1..=(start + 7).min(n) {
                if !bytes[end - 1].is_ascii_lowercase() {
                    break;
                }
                let observed = &bytes[start..end];
                let min_len = observed.len().saturating_sub(2);
                let max_len = (observed.len() + 2).min(7);
                if let Ok(observed_text) = std::str::from_utf8(observed) {
                    if exact_inventory.contains(observed_text) {
                        edges[start].push(SyllableEdge {
                            start,
                            end,
                            syllable: observed_text.to_owned(),
                            cost: 0.0,
                        });
                        // Exact token matches need no edit-distance work. Keep configured
                        // fuzzy phonetic alternatives (z/zh, an/ang, and similar) so
                        // dialect preferences still compete with the exact path.
                        for &syllable in inventory.iter().take(max_len + 1).skip(min_len).flatten()
                        {
                            if syllable != observed_text {
                                if let Some(cost) =
                                    fuzzy_cost(observed, syllable.as_bytes(), config)
                                {
                                    edges[start].push(SyllableEdge {
                                        start,
                                        end,
                                        syllable: syllable.to_owned(),
                                        cost,
                                    });
                                }
                            }
                        }
                        continue;
                    }
                }
                for &syllable in inventory.iter().take(max_len + 1).skip(min_len).flatten() {
                    let mut cost =
                        weighted_osa(observed, syllable.as_bytes(), config.max_edge_cost);
                    if let Some(fuzzy) = fuzzy_cost(observed, syllable.as_bytes(), config) {
                        cost = cost.min(fuzzy);
                    }
                    if cost <= config.max_edge_cost {
                        edges[start].push(SyllableEdge {
                            start,
                            end,
                            syllable: syllable.to_owned(),
                            cost,
                        });
                    }
                }
            }
        }
        // Mark alphabetic spans that have no plausible syllable edge as English.
        let mut i = 0;
        while i < n {
            if !bytes[i].is_ascii_alphabetic() {
                i += 1;
                continue;
            }
            let begin = i;
            while i < n && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let mut reachable = vec![false; i - begin + 1];
            reachable[0] = true;
            for p in begin..i {
                if reachable[p - begin] {
                    for edge in &edges[p] {
                        if edge.end <= i && edge.cost <= config.exact_cost {
                            reachable[edge.end - begin] = true;
                        }
                    }
                }
            }
            if !reachable[i - begin] {
                english[begin..i].fill(true);
            }
        }
        for list in &mut edges {
            list.sort_by(|a, b| {
                a.end
                    .cmp(&b.end)
                    .then_with(|| a.cost.total_cmp(&b.cost))
                    .then_with(|| a.syllable.cmp(&b.syllable))
            });
            let mut bounded = Vec::with_capacity(list.len().min(28));
            let mut current_end = usize::MAX;
            let mut count = 0;
            for edge in list.drain(..) {
                if edge.end != current_end {
                    current_end = edge.end;
                    count = 0;
                }
                if count < 4 {
                    bounded.push(edge);
                    count += 1;
                }
            }
            bounded.sort_by(|a, b| {
                a.cost
                    .total_cmp(&b.cost)
                    .then_with(|| a.end.cmp(&b.end))
                    .then_with(|| a.syllable.cmp(&b.syllable))
            });
            *list = bounded;
        }
        Self {
            input: s,
            original: input.to_owned(),
            edges,
            english,
            config: config.clone(),
        }
    }
}

fn fuzzy_cost(observed: &[u8], target: &[u8], c: &PinyinConfig) -> Option<f32> {
    let initials = [
        (b"zh".as_slice(), b"z".as_slice()),
        (b"ch", b"c"),
        (b"sh", b"s"),
        (b"l", b"n"),
        (b"f", b"h"),
    ];
    for (a, b) in initials {
        if (observed.starts_with(a)
            && target.starts_with(b)
            && observed[a.len()..] == target[b.len()..])
            || (observed.starts_with(b)
                && target.starts_with(a)
                && observed[b.len()..] == target[a.len()..])
        {
            return Some(c.fuzzy_cost);
        }
    }
    let finals = [
        (b"ang".as_slice(), b"an".as_slice()),
        (b"eng", b"en"),
        (b"ing", b"in"),
    ];
    for (a, b) in finals {
        if (observed.ends_with(a)
            && target.ends_with(b)
            && observed[..observed.len() - a.len()] == target[..target.len() - b.len()])
            || (observed.ends_with(b)
                && target.ends_with(a)
                && observed[..observed.len() - b.len()] == target[..target.len() - a.len()])
        {
            return Some(c.fuzzy_cost);
        }
    }
    None
}

pub(crate) fn syllable_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| syllables().iter().copied().collect())
}

fn syllables_by_length() -> &'static [Vec<&'static str>; 8] {
    static INDEX: OnceLock<[Vec<&'static str>; 8]> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut groups: [Vec<&'static str>; 8] = std::array::from_fn(|_| Vec::new());
        for syllable in syllables() {
            if syllable.len() < groups.len() {
                groups[syllable.len()].push(*syllable);
            }
        }
        groups
    })
}

fn syllables() -> &'static [&'static str] {
    // Compact built-in segmentation inventory; dictionaries remain external and may extend it.
    static INVENTORY: OnceLock<Vec<&'static str>> = OnceLock::new();
    INVENTORY.get_or_init(||"a ai an ang ao ba bai ban bang bao bei ben beng bi bian biao bie bin bing bo bu ca cai can cang cao ce cen ceng cha chai chan chang chao che chen cheng chi chong chou chu chua chuai chuan chuang chui chun chuo ci cong cou cu cuan cui cun cuo da dai dan dang dao de dei den deng di dia dian diao die ding diu dong dou du duan dui dun duo e ei en eng er fa fan fang fei fen feng fo fou fu ga gai gan gang gao ge gei gen geng gong gou gu gua guai guan guang gui gun guo ha hai han hang hao he hei hen heng hong hou hu hua huai huan huang hui hun huo ji jia jian jiang jiao jie jin jing jiong jiu ju juan jue jun ka kai kan kang kao ke ken keng kong kou ku kua kuai kuan kuang kui kun kuo la lai lan lang lao le lei leng li lia lian liang liao lie lin ling liu long lou lu luan lun luo lv lve ma mai man mang mao me mei men meng mi mian miao mie min ming miu mo mou mu na nai nan nang nao ne nei nen neng ni nian niang niao nie nin ning niu nong nou nu nuan nuo nv nve o ou pa pai pan pang pao pei pen peng pi pian piao pie pin ping po pou pu qi qia qian qiang qiao qie qin qing qiong qiu qu quan que qun ran rang rao re ren reng ri rong rou ru ruan rui run ruo sa sai san sang sao se sen seng sha shai shan shang shao she shei shen sheng shi shou shu shua shuai shuan shuang shui shun shuo si song sou su suan sui sun suo ta tai tan tang tao te teng ti tian tiao tie ting tong tou tu tuan tui tun tuo wa wai wan wang wei wen weng wo wu xi xia xian xiang xiao xie xin xing xiong xiu xu xuan xue xun ya yan yang yao ye yi yin ying yo yong you yu yuan yue yun za zai zan zang zao ze zei zen zeng zha zhai zhan zhang zhao zhe zhei zhen zheng zhi zhong zhou zhu zhua zhuai zhuan zhuang zhui zhun zhuo zi zou zu zuan zui zun zuo".split_whitespace().collect())
}
