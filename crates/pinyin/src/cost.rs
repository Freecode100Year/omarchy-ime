pub(crate) fn key_cost(a: u8, b: u8) -> f32 {
    if a == b {
        return 0.0;
    }
    let rows: [&[u8]; 3] = [b"qwertyuiop", b"asdfghjkl", b"zxcvbnm"];
    let pos = |c: u8| -> Option<(i32, i32)> {
        for (y, row) in rows.iter().enumerate() {
            if let Some(x) = row.iter().position(|v| *v == c) {
                return Some((
                    x as i32
                        + if y == 1 {
                            1
                        } else if y == 2 {
                            2
                        } else {
                            0
                        },
                    y as i32,
                ));
            }
        }
        None
    };
    match (pos(a), pos(b)) {
        (Some((x1, y1)), Some((x2, y2))) if (x1 - x2).abs() <= 1 && (y1 - y2).abs() <= 1 => 0.55,
        _ => 1.0,
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
