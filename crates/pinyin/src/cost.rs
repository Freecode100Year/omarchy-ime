const KEY_COORDS: [(i8, i8); 26] = [
    (1, 1), // a
    (5, 2), // b
    (4, 2), // c
    (3, 1), // d
    (2, 0), // e
    (4, 1), // f
    (5, 1), // g
    (6, 1), // h
    (7, 0), // i
    (7, 1), // j
    (8, 1), // k
    (9, 1), // l
    (8, 2), // m
    (7, 2), // n
    (8, 0), // o
    (9, 0), // p
    (0, 0), // q
    (3, 0), // r
    (1, 1), // s
    (4, 0), // t
    (6, 0), // u
    (3, 2), // v
    (1, 0), // w
    (2, 2), // x
    (5, 0), // y
    (1, 2), // z
];

pub(crate) fn key_cost(a: u8, b: u8) -> f32 {
    if a == b {
        return 0.0;
    }
    let Some(&(ax, ay)) = a.checked_sub(b'a').and_then(|i| KEY_COORDS.get(i as usize)) else {
        return 1.0;
    };
    let Some(&(bx, by)) = b.checked_sub(b'a').and_then(|i| KEY_COORDS.get(i as usize)) else {
        return 1.0;
    };
    if (ax - bx).abs() <= 1 && (ay - by).abs() <= 1 {
        0.55
    } else {
        1.0
    }
}

/// Optimal-string-alignment distance with keyboard-weighted substitutions.
pub(crate) fn weighted_osa(a: &[u8], b: &[u8], cap: f32) -> f32 {
    let (n, m) = (a.len(), b.len());
    if n.abs_diff(m) > 2 || n > 7 || m > 7 {
        return cap + 1.0;
    }
    let mut d = [[f32::INFINITY; 8]; 8];
    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
        row[0] = i as f32 * 0.85;
    }
    for (j, value) in d[0].iter_mut().enumerate().take(m + 1) {
        *value = j as f32 * 0.85;
    }
    for i in 1..=n {
        for j in 1..=m {
            let sub = if a[i - 1] == b[j - 1] {
                0.0
            } else {
                key_cost(a[i - 1], b[j - 1])
            };
            d[i][j] = (d[i - 1][j] + 0.85)
                .min(d[i][j - 1] + 0.85)
                .min(d[i - 1][j - 1] + sub);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 0.65);
            }
        }
    }
    d[n][m]
}

/// Word-level OSA distance for concatenated multi-syllable keys.
pub(crate) fn weighted_word_osa(a: &[u8], b: &[u8], max_len_delta: usize) -> Option<f32> {
    if a.len().abs_diff(b.len()) > max_len_delta {
        return None;
    }
    let width = b.len() + 1;
    let mut d = vec![f32::INFINITY; (a.len() + 1) * width];
    for i in 0..=a.len() {
        d[i * width] = i as f32 * 0.85;
    }
    for (j, value) in d.iter_mut().take(b.len() + 1).enumerate() {
        *value = j as f32 * 0.85;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let sub = if a[i - 1] == b[j - 1] {
                0.0
            } else {
                key_cost(a[i - 1], b[j - 1])
            };
            let idx = i * width + j;
            d[idx] = (d[(i - 1) * width + j] + 0.85)
                .min(d[i * width + j - 1] + 0.85)
                .min(d[(i - 1) * width + j - 1] + sub);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[idx] = d[idx].min(d[(i - 2) * width + j - 2] + 0.65);
            }
        }
    }
    Some(d[a.len() * width + b.len()])
}

#[cfg(test)]
mod tests {
    use super::{key_cost, weighted_word_osa};
    #[test]
    fn keyboard_distance_uses_physical_qwerty_adjacency() {
        assert_eq!(key_cost(b'f', b'g'), 0.55);
        assert_eq!(key_cost(b'q', b'a'), 0.55);
        assert_eq!(key_cost(b'q', b'm'), 1.0);
        assert_eq!(key_cost(b'f', b'f'), 0.0);
        let distance = weighted_word_osa(b"shuansghou", b"shuangshou", 2).unwrap();
        assert!(distance <= 0.65);
    }
}
