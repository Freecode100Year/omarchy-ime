use wubi::{parse_tsv, WubiLexicon, WubiVersion};

#[test]
fn parses_versioned_wubi_rows_and_indexes_reverse_lookup() {
    let lexicon = parse_tsv("86\tzhong\t中\t100\n98\tabc\t汉\t20\n").unwrap();
    assert_eq!(lexicon.lookup(WubiVersion::V86, "zhong")[0].text, "中");
    assert_eq!(lexicon.reverse_lookup(WubiVersion::V98, '汉'), ["abc"]);
    assert!(lexicon.lookup(WubiVersion::NewCentury, "zhong").is_empty());
}

#[test]
fn reverse_lookup_ignores_characters_inside_phrases() {
    let lexicon = parse_tsv("86\ta\t中国\t100\n86\tb\t中\t10\n").unwrap();
    assert_eq!(lexicon.reverse_lookup(WubiVersion::V86, '中'), ["b"]);
}

#[test]
fn rejects_malformed_or_empty_wubi_tables() {
    assert!(parse_tsv("# table\n").is_err());
    assert!(parse_tsv("86\tab\t\t10\n").is_err());
    assert!(parse_tsv("12\tab\t字\t1\n").is_err());
}

#[test]
fn prefix_lookup_finds_longer_codes_without_losing_exact_rows() {
    let lexicon = parse_tsv("86\tzhong\t中\t100\n86\tzhongg\t中国\t90\n").unwrap();
    assert_eq!(lexicon.lookup(WubiVersion::V86, "zhong").len(), 2);
}

#[test]
fn one_letter_prefix_keeps_high_frequency_codes_beyond_the_first_64_rows() {
    let mut text = String::new();
    for first in b'a'..=b'c' {
        for second in b'a'..=b'z' {
            if text.lines().count() == 64 {
                break;
            }
            text.push_str(&format!(
                "86\ta{}{}\t低\t1\n",
                first as char, second as char
            ));
        }
        if text.lines().count() == 64 {
            break;
        }
    }
    text.push_str("86\tazz\t高\t9999\n");
    let lexicon = parse_tsv(&text).unwrap();
    assert_eq!(lexicon.lookup(WubiVersion::V86, "a")[0].text, "高");
}
