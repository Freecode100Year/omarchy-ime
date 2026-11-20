use pinyin::{PinyinConfig, SyllableDag};

#[test]
fn syllable_dag_contains_exact_edge() {
    let dag = SyllableDag::from_input("ni", &PinyinConfig::default());
    assert!(dag.edges[0]
        .iter()
        .any(|edge| edge.syllable == "ni" && edge.end == 2 && edge.cost == 0.0));
}
