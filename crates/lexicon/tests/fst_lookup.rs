use lexicon::{read_pinyin_tsv, read_tsv, FstPinyinLexicon, FstWubiLexicon};
use pinyin::{Lexeme, PinyinLexicon};
use wubi::{WubiLexicon, WubiVersion};

#[test]
fn pinyin_keys_preserve_syllable_boundaries() {
    let rows = read_pinyin_tsv("xian\t先\t10\nxi an\t西安\t20\n").unwrap();
    let dictionary = FstPinyinLexicon::from_sorted(rows).unwrap();
    assert_eq!(dictionary.lookup(&["xian".into()])[0].text, "先");
    assert_eq!(
        dictionary.lookup(&["xi".into(), "an".into()])[0].text,
        "西安"
    );
}

#[test]
fn rejects_empty_candidate_rows() {
    assert!(read_tsv("aaaa\t\t1\n").is_err());
}

#[test]
fn character_reverse_lookup_does_not_return_phrase_codes() {
    let dictionary = FstWubiLexicon::from_version_rows(vec![(
        WubiVersion::V86,
        vec![
            (
                "abcd".into(),
                Lexeme {
                    text: "中国".into(),
                    frequency: 10.0,
                    annotation: None,
                },
            ),
            (
                "efgh".into(),
                Lexeme {
                    text: "中".into(),
                    frequency: 5.0,
                    annotation: None,
                },
            ),
        ],
    )])
    .unwrap();
    assert_eq!(
        dictionary.reverse_lookup(WubiVersion::V86, '中'),
        vec!["efgh".to_owned()]
    );
}
