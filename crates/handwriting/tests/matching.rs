use handwriting::{extract_features, match_templates, HandwritingTemplate, StrokePoint};

#[test]
fn empty_and_degenerate_strokes_have_no_features() {
    assert!(extract_features(&[]).is_none());
    assert!(extract_features(&[
        StrokePoint {
            x: 1.0,
            y: 2.0,
            stroke_id: 0
        },
        StrokePoint {
            x: 1.0,
            y: 2.0,
            stroke_id: 0
        }
    ])
    .is_none());
}

#[test]
fn extreme_finite_coordinates_stay_finite() {
    let feature = extract_features(&[
        StrokePoint {
            x: -f32::MAX,
            y: 0.0,
            stroke_id: 0,
        },
        StrokePoint {
            x: f32::MAX,
            y: 1.0,
            stroke_id: 0,
        },
    ])
    .unwrap();
    let matches = match_templates(
        &feature,
        &[HandwritingTemplate {
            character: '一',
            features: feature.clone(),
            frequency: 1.0,
        }],
        5,
    );
    assert_eq!(matches.first().map(|m| m.character), Some('一'));
    assert!(matches
        .iter()
        .all(|m| m.distance.is_finite() && m.score.is_finite()));
}
