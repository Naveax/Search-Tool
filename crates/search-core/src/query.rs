use crate::store::{normalize_name, FLAG_DIRECTORY, FLAG_HIDDEN, FLAG_SYSTEM};

pub fn relevance_score(query: &str, name: &str, flags: u16) -> i32 {
    let q = normalize_name(query);
    let n = normalize_name(name);
    let stem = n.rsplit_once('.').map_or(n.as_str(), |(stem, _)| stem);
    let mut score = 0_i32;
    if n == q {
        score += 10_000;
    } else if stem == q {
        score += 9_000;
    } else if n.starts_with(&q) {
        score += 7_000;
    } else if stem.starts_with(&q) {
        score += 6_500;
    } else if n.contains(&q) {
        score += 4_000;
    }
    if flags & FLAG_DIRECTORY != 0 {
        score += 120;
    }
    if flags & FLAG_HIDDEN != 0 {
        score -= 300;
    }
    if flags & FLAG_SYSTEM != 0 {
        score -= 500;
    }
    score - (n.len().saturating_sub(q.len()).min(500) as i32)
}

pub fn fuzzy_distance(query: &str, name: &str, max_distance: usize) -> Option<usize> {
    let q = normalize_name(query);
    let n = normalize_name(name);
    let stem = n.rsplit_once('.').map_or(n.as_str(), |(stem, _)| stem);
    let a = q.as_bytes();
    let d_name = bounded_levenshtein(a, n.as_bytes(), max_distance);
    let d_stem = bounded_levenshtein(a, stem.as_bytes(), max_distance);
    match (d_name, d_stem) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

pub fn fuzzy_seed(query: &str) -> String {
    let normalized = normalize_name(query);
    let chars: Vec<char> = normalized.chars().collect();
    if chars.len() <= 2 {
        normalized
    } else {
        chars.into_iter().take(2).collect()
    }
}

pub fn bounded_levenshtein(a: &[u8], b: &[u8], max_distance: usize) -> Option<usize> {
    if a.len().abs_diff(b.len()) > max_distance {
        return None;
    }
    if a == b {
        return Some(0);
    }
    if a.is_empty() {
        return (b.len() <= max_distance).then_some(b.len());
    }
    if b.is_empty() {
        return (a.len() <= max_distance).then_some(a.len());
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0_usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        let mut row_min = curr[0];
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            row_min = row_min.min(curr[j + 1]);
        }
        if row_min > max_distance {
            return None;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    (prev[b.len()] <= max_distance).then_some(prev[b.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_distance_handles_typos() {
        assert_eq!(bounded_levenshtein(b"node", b"nod", 2), Some(1));
        assert_eq!(bounded_levenshtein(b"node", b"xode", 2), Some(1));
        assert_eq!(bounded_levenshtein(b"node", b"python", 2), None);
    }

    #[test]
    fn extension_does_not_ruin_fuzzy_match() {
        assert_eq!(fuzzy_distance("notpad", "notepad.exe", 2), Some(1));
    }
}
