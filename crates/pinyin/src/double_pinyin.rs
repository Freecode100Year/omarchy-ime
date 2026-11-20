//! Xiaohe and Natural Code double-pinyin maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoublePinyinScheme {
    Xiaohe,
    NaturalCode,
}

/// Expand complete key pairs to canonical pinyin. A trailing odd key is kept
/// out of the decoded prefix so callers can retain it as the next initial.
pub fn expand_double_pinyin(input: &str, scheme: DoublePinyinScheme) -> Option<String> {
    if !input.is_ascii() {
        return None;
    }
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len() * 2);
    for index in (0..bytes.len() / 2).map(|pair| pair * 2) {
        let first = bytes[index].to_ascii_lowercase() as char;
        let second = bytes[index + 1].to_ascii_lowercase() as char;
        let initial = initial_for(first)?;
        let syllable = if initial.is_empty() && is_zero_initial_alias(first, second) {
            format!("{first}{second}")
        } else {
            let final_part = final_for(second, first, scheme)?;
            if initial.is_empty() {
                // In these layouts a/e/o act as zero-initial keys. The
                // canonical syllable spellings collapse duplicated vowels.
                let mut value = format!("{first}{}", final_part.replace('ü', "v"));
                if value.as_bytes().get(1) == Some(&(first as u8)) {
                    value.remove(1);
                }
                value
            } else {
                format!("{initial}{}", final_part.replace('ü', "v"))
            }
        };
        output.push_str(&syllable);
    }
    Some(output)
}

/// Encode one canonical pinyin syllable with the selected layout. This inverse
/// is also used by table-driven tests so all supported syllables round-trip.
pub fn encode_double_pinyin(syllable: &str, scheme: DoublePinyinScheme) -> Option<String> {
    if !syllable.is_ascii() || syllable.is_empty() {
        return None;
    }
    let syllable = syllable.to_ascii_lowercase();
    let (initial_key, initial, final_part) = if let Some(rest) = syllable.strip_prefix("zh") {
        ('v', "zh", rest)
    } else if let Some(rest) = syllable.strip_prefix("ch") {
        ('i', "ch", rest)
    } else if let Some(rest) = syllable.strip_prefix("sh") {
        ('u', "sh", rest)
    } else if let Some(first) = syllable
        .chars()
        .next()
        .filter(|c| "bpmfdtnlgkhjqxzcsrwy".contains(*c))
    {
        (
            first,
            &syllable[..first.len_utf8()],
            &syllable[first.len_utf8()..],
        )
    } else if let Some(first @ ('a' | 'e' | 'o')) = syllable.chars().next() {
        (first, "", syllable.as_str())
    } else {
        return None;
    };
    let final_part = match final_part {
        "v" | "ve" => {
            if final_part == "v" {
                "ü"
            } else {
                "üe"
            }
        }
        "u" if matches!(initial, "j" | "q" | "x" | "y") => "ü",
        rest => rest,
    };
    let final_key = key_for_final(final_part, initial_key, scheme)?;
    Some(format!("{initial_key}{final_key}"))
}

fn initial_for(key: char) -> Option<&'static str> {
    Some(match key {
        'a' | 'e' | 'o' => "",
        'v' => "zh",
        'i' => "ch",
        'u' => "sh",
        'b'..='z' => {
            return Some(match key {
                'a'..='z' => INITIALS[key as usize - 'a' as usize],
                _ => "",
            })
        }
        _ => return None,
    })
}

const INITIALS: [&str; 26] = [
    "", "b", "c", "d", "", "f", "g", "h", "", "j", "k", "l", "m", "n", "", "p", "q", "r", "s", "t",
    "", "", "w", "x", "y", "z",
];

fn is_zero_initial_alias(first: char, second: char) -> bool {
    matches!(first, 'a' | 'e' | 'o')
        && matches!(
            (first, second),
            ('a', 'i' | 'n' | 'o') | ('e', 'i' | 'n' | 'r') | ('o', 'u' | 'n')
        )
}

fn in_set(initial_key: char, values: &str) -> bool {
    values.contains(initial_key)
}

fn final_for(key: char, initial_key: char, scheme: DoublePinyinScheme) -> Option<&'static str> {
    let value = match scheme {
        DoublePinyinScheme::Xiaohe => match key {
            'a' => "a",
            'o' => {
                if in_set(initial_key, "dtnlgkhjqxyvuirzcs") {
                    "uo"
                } else {
                    "o"
                }
            }
            'e' => "e",
            'i' => "i",
            'u' => "u",
            'v' => {
                if in_set(initial_key, "dtgkhvuirzcs") {
                    "ui"
                } else if in_set(initial_key, "jqxy") {
                    "u"
                } else if in_set(initial_key, "nl") {
                    "ü"
                } else {
                    "ue"
                }
            }
            'd' => "ai",
            'w' => "ei",
            'c' => "ao",
            'z' => "ou",
            'j' => "an",
            'f' => "en",
            'h' => "ang",
            'g' => "eng",
            's' => {
                if in_set(initial_key, "jqx") {
                    "iong"
                } else {
                    "ong"
                }
            }
            'r' => {
                if in_set(initial_key, "aeo") {
                    "er"
                } else {
                    "uan"
                }
            }
            'x' => {
                if in_set(initial_key, "gkhvuirzcs") {
                    "ua"
                } else {
                    "ia"
                }
            }
            'p' => "ie",
            'n' => "iao",
            'q' => "iu",
            'm' => "ian",
            'b' => "in",
            'k' => {
                if in_set(initial_key, "gkhvuirzcs") {
                    "uai"
                } else {
                    "ing"
                }
            }
            'l' => {
                if in_set(initial_key, "jqxnl") {
                    "iang"
                } else {
                    "uang"
                }
            }
            'y' => "un",
            't' => {
                if in_set(initial_key, "nl") {
                    "üe"
                } else {
                    "ue"
                }
            }
            _ => return None,
        },
        DoublePinyinScheme::NaturalCode => match key {
            'a' => "a",
            'o' => {
                if in_set(initial_key, "dtnlgkhjqxyvuirzcs") {
                    "uo"
                } else {
                    "o"
                }
            }
            'e' => "e",
            'i' => "i",
            'u' => "u",
            'l' => "ai",
            'j' => "an",
            'h' => "ang",
            'k' => "ao",
            'z' => "ei",
            'f' => "en",
            'g' => "eng",
            'r' => {
                if in_set(initial_key, "aeo") {
                    "er"
                } else {
                    "uan"
                }
            }
            'w' => {
                if in_set(initial_key, "gkhvuirzcs") {
                    "ua"
                } else {
                    "ia"
                }
            }
            'm' => "ian",
            'd' => {
                if in_set(initial_key, "jqxnl") {
                    "iang"
                } else {
                    "uang"
                }
            }
            'c' => "iao",
            'x' => "ie",
            'n' => "in",
            'y' => {
                if in_set(initial_key, "gkhvuirzcs") {
                    "uai"
                } else {
                    "ing"
                }
            }
            's' => {
                if in_set(initial_key, "jqx") {
                    "iong"
                } else {
                    "ong"
                }
            }
            'q' => "iu",
            'b' => "ou",
            't' => {
                if in_set(initial_key, "nl") {
                    "üe"
                } else {
                    "ue"
                }
            }
            'p' => "un",
            'v' => {
                if in_set(initial_key, "dtgkhvuirzcs") {
                    "ui"
                } else if in_set(initial_key, "jqxy") {
                    "u"
                } else if in_set(initial_key, "nl") {
                    "ü"
                } else {
                    "ue"
                }
            }
            _ => return None,
        },
    };
    Some(value)
}

fn key_for_final(final_part: &str, initial_key: char, scheme: DoublePinyinScheme) -> Option<char> {
    if final_part == "ü" && in_set(initial_key, "jqxy") {
        return Some('v');
    }
    let candidates = match scheme {
        DoublePinyinScheme::Xiaohe => &[
            ("a", 'a'),
            ("o", 'o'),
            ("e", 'e'),
            ("i", 'i'),
            ("u", 'u'),
            ("ü", 'v'),
            ("ue", 't'),
            ("üe", 't'),
            ("üe", 't'),
            ("ai", 'd'),
            ("ei", 'w'),
            ("ao", 'c'),
            ("ou", 'z'),
            ("an", 'j'),
            ("en", 'f'),
            ("ang", 'h'),
            ("eng", 'g'),
            ("ong", 's'),
            ("iong", 's'),
            ("iao", 'n'),
            ("iu", 'q'),
            ("ian", 'm'),
            ("in", 'b'),
            ("ing", 'k'),
            ("uai", 'k'),
            ("iang", 'l'),
            ("uang", 'l'),
            ("uan", 'r'),
            ("un", 'y'),
            ("uo", 'o'),
            ("ia", 'x'),
            ("ua", 'x'),
            ("ie", 'p'),
            ("ui", 'v'),
            ("er", 'r'),
        ][..],
        DoublePinyinScheme::NaturalCode => &[
            ("a", 'a'),
            ("o", 'o'),
            ("e", 'e'),
            ("i", 'i'),
            ("u", 'u'),
            ("ü", 'v'),
            ("ue", 'v'),
            ("üe", 'v'),
            ("ue", 't'),
            ("üe", 't'),
            ("ai", 'l'),
            ("ei", 'z'),
            ("ao", 'k'),
            ("ou", 'b'),
            ("an", 'j'),
            ("en", 'f'),
            ("ang", 'h'),
            ("eng", 'g'),
            ("ong", 's'),
            ("iong", 's'),
            ("iao", 'c'),
            ("iu", 'q'),
            ("ian", 'm'),
            ("in", 'n'),
            ("ing", 'y'),
            ("uai", 'y'),
            ("iang", 'd'),
            ("uang", 'd'),
            ("uan", 'r'),
            ("un", 'p'),
            ("uo", 'o'),
            ("ia", 'w'),
            ("ua", 'w'),
            ("ie", 'x'),
            ("ui", 'v'),
            ("er", 'r'),
        ][..],
    };
    candidates
        .iter()
        .find(|(name, key)| {
            *name == final_part && final_for(*key, initial_key, scheme) == Some(final_part)
        })
        .map(|(_, key)| *key)
}
