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

#[test]
fn reverse_direction_strokes_match_the_same_character() {
    let points = |xs: [f32; 3]| {
        xs.into_iter()
            .map(|x| StrokePoint {
                x,
                y: 0.0,
                stroke_id: 0,
            })
            .collect::<Vec<_>>()
    };
    let forward = extract_features(&points([0.0, 50.0, 100.0])).unwrap();
    let reverse = extract_features(&points([100.0, 50.0, 0.0])).unwrap();
    let matches = match_templates(
        &forward,
        &[HandwritingTemplate {
            character: '一',
            features: reverse,
            frequency: 1.0,
        }],
        1,
    );
    assert!(
        matches[0].distance < 0.1,
        "distance {}",
        matches[0].distance
    );
}
