use crate::FeatureVector;

#[derive(Clone, Debug)]
pub struct HandwritingTemplate {
    pub character: char,
    pub features: FeatureVector,
    pub frequency: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HandwritingMatch {
    pub character: char,
    pub distance: f32,
    pub score: f32,
}

/// Rank templates by a finite weighted feature distance. Smaller distance is better.
pub fn match_templates(
    query: &FeatureVector,
    templates: &[HandwritingTemplate],
    limit: usize,
) -> Vec<HandwritingMatch> {
    let mut out: Vec<_> = templates
        .iter()
        .map(|t| {
            let grid = query
                .grid
                .iter()
                .zip(t.features.grid.iter())
                .map(|(a, b)| (a - b).abs())
                .sum::<f32>()
                / 64.0;
            let dir = direction_distance(&query.directions, &t.features.directions);
            let stroke = (query.stroke_count.abs_diff(t.features.stroke_count) as f32)
                / (query.stroke_count.max(t.features.stroke_count).max(1) as f32);
            let mut reversed = t.features.trajectory.clone();
            reversed.reverse();
            let trajectory = dtw(&query.trajectory, &t.features.trajectory)
                .min(dtw(&query.trajectory, &reversed));
            let distance = grid * 0.35 + dir * 0.20 + stroke * 0.10 + trajectory * 0.35;
            let score = (1.0 - distance).max(0.0) + t.frequency.max(0.0).ln_1p() * 0.001;
            HandwritingMatch {
                character: t.character,
                distance,
                score,
            }
        })
        .filter(|m| m.distance.is_finite() && m.score.is_finite())
        .collect();
    out.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.character.cmp(&b.character))
    });
    out.truncate(limit);
    out
}

fn direction_distance(a: &[f32; 8], b: &[f32; 8]) -> f32 {
    let direct = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .sum::<f32>();
    let reversed = a
        .iter()
        .enumerate()
        .map(|(i, x)| (x - b[(i + 4) % 8]).abs())
        .sum::<f32>();
    direct.min(reversed) / 2.0
}

fn dtw(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 1.0;
    }
    let inf = f32::INFINITY;
    let mut prev = vec![inf; b.len() + 1];
    prev[0] = 0.0;
    for (i, pa) in a.iter().enumerate() {
        let mut curr = vec![inf; b.len() + 1];
        let band = a.len().max(b.len());
        let lo = (i + 1).saturating_sub(band).max(1);
        let hi = (i + 1 + band).min(b.len());
        for j in lo..=hi {
            let pb = b[j - 1];
            let dx = pa[0] - pb[0];
            let dy = pa[1] - pb[1];
            let stroke_cost = if pa[2] == pb[2] { 0.0 } else { 0.25 };
            let local = (dx * dx + dy * dy).sqrt() + stroke_cost;
            curr[j] = local + prev[j].min(curr[j - 1]).min(prev[j - 1]);
        }
        prev = curr;
    }
    (prev[b.len()] / a.len().max(b.len()) as f32).min(1.0)
}
