use pinyin::{PinyinConfig, SyllableDag};

#[test]
fn syllable_dag_contains_exact_edge() {
    let dag = SyllableDag::from_input("ni", &PinyinConfig::default());
    assert!(dag.edges[0]
        .iter()
        .any(|edge| edge.syllable == "ni" && edge.end == 2 && edge.cost == 0.0));
}

#[test]
fn correction_graph_bounds_alternatives_per_span() {
    let dag = SyllableDag::from_input("nihao", &PinyinConfig::default());
    for start in 0..dag.edges.len() {
        for end in start + 1..=dag.input.len() {
            assert!(
                dag.edges[start]
                    .iter()
                    .filter(|edge| edge.end == end)
                    .count()
                    <= 4
            );
        }
    }
}

#[test]
fn exact_syllables_keep_fuzzy_phonetic_alternatives() {
    let dag = SyllableDag::from_input("zao", &PinyinConfig::default());
    assert!(dag.edges[0]
        .iter()
        .any(|edge| edge.syllable == "zao" && edge.cost == 0.0));
    assert!(dag.edges[0]
        .iter()
        .any(|edge| edge.syllable == "zhao" && edge.cost > 0.0));
}
