#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub stroke_id: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatureVector {
    pub grid: [f32; 64],
    pub directions: [f32; 8],
    pub stroke_count: u16,
    pub trajectory: Vec<[f32; 3]>,
}

/// Normalize a stroke set into an 8x8 occupancy grid and 8-bin direction histogram.
pub fn extract_features(points: &[StrokePoint]) -> Option<FeatureVector> {
    let valid: Vec<_> = points
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    if valid.is_empty() {
        return None;
    }
    let min_x = valid
        .iter()
        .map(|p| p.x as f64)
        .fold(f64::INFINITY, f64::min);
    let max_x = valid
        .iter()
        .map(|p| p.x as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = valid
        .iter()
        .map(|p| p.y as f64)
        .fold(f64::INFINITY, f64::min);
    let max_y = valid
        .iter()
        .map(|p| p.y as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let w = max_x - min_x;
    let h = max_y - min_y;
    if !w.is_finite() || !h.is_finite() || w.max(h) <= f64::EPSILON {
        return None;
    }
    let scale = w.max(h);
    let ox = (1.0 - w / scale) * 0.5;
    let oy = (1.0 - h / scale) * 0.5;
    let norm = |p: StrokePoint| {
        (
            ((p.x as f64 - min_x) / scale + ox) as f32,
            ((p.y as f64 - min_y) / scale + oy) as f32,
        )
    };
    let mut grid = [0.0; 64];
    let mut directions = [0.0; 8];
    for p in &valid {
        let (x, y) = norm(*p);
        let ix = ((x * 7.999).floor() as usize).min(7);
        let iy = ((y * 7.999).floor() as usize).min(7);
        grid[iy * 8 + ix] = 1.0;
    }
    let mut segments = 0usize;
    for pair in valid.windows(2) {
        if pair[0].stroke_id != pair[1].stroke_id {
            continue;
        }
        let dx = pair[1].x as f64 - pair[0].x as f64;
        let dy = pair[1].y as f64 - pair[0].y as f64;
        if dx.abs() + dy.abs() <= f64::EPSILON {
            continue;
        }
        let angle = dy.atan2(dx).rem_euclid(std::f64::consts::TAU);
        let bin = ((angle / std::f64::consts::TAU * 8.0).floor() as usize).min(7);
        directions[bin] += 1.0;
        segments += 1;
    }
    if segments > 0 {
        for d in &mut directions {
            *d /= segments as f32;
        }
    }
    let stroke_count = valid
        .iter()
        .map(|p| p.stroke_id)
        .collect::<std::collections::HashSet<_>>()
        .len()
        .min(u16::MAX as usize) as u16;
    let take = valid.len().min(128);
    let trajectory = (0..take)
        .map(|i| {
            let index = if take <= 1 {
                0
            } else {
                i * (valid.len() - 1) / (take - 1)
            };
            let (x, y) = norm(valid[index]);
            [x, y, valid[index].stroke_id as f32]
        })
        .collect();
    Some(FeatureVector {
        grid,
        directions,
        stroke_count,
        trajectory,
    })
}
