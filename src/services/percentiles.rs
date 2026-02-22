/// Percentile helpers for already-sorted slices.
///
/// - Empty input => `None` (or `0.0` for the f32 convenience wrapper).
/// - `percentile <= 0` => first element.
/// - `percentile >= 100` => last element.
/// - Otherwise we compute a position within `[0, len-1]` and round to the
///   nearest index.

/// Returns the percentile value from a slice that is already sorted in
/// ascending order.
pub fn get_percentile_value<T: Copy>(sorted_values: &[T], percentile: f64) -> Option<T> {
    if sorted_values.is_empty() {
        return None;
    }

    let index = if percentile <= 0.0 {
        0
    } else if percentile >= 100.0 {
        sorted_values.len() - 1
    } else {
        let position = (percentile / 100.0) * (sorted_values.len() as f64 - 1.0);
        position.round() as usize
    };

    sorted_values.get(index).copied()
}

/// Returns the index into a sorted slice for the given percentile.
///
/// This is useful when you want to select a percentile from a slice of non-`Copy`
/// values without cloning the selected element.
pub fn get_percentile_index(len: usize, percentile: f64) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let index = if percentile <= 0.0 {
        0
    } else if percentile >= 100.0 {
        len - 1
    } else {
        let position = (percentile / 100.0) * (len as f64 - 1.0);
        position.round() as usize
    };

    Some(index)
}

/// Convenience wrapper for `f32` results.
pub fn get_percentile_value_f32(sorted_values: &[f32], percentile: f64) -> f32 {
    get_percentile_value(sorted_values, percentile).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_sorted_returns_none_for_empty_input() {
        let values: [i32; 0] = [];
        assert_eq!(get_percentile_value(&values, 50.0), None);
    }

    #[test]
    fn value_sorted_clamps_to_first_and_last() {
        let values = [10, 20, 30];
        assert_eq!(get_percentile_value(&values, -1.0), Some(10));
        assert_eq!(get_percentile_value(&values, 0.0), Some(10));
        assert_eq!(get_percentile_value(&values, 100.0), Some(30));
        assert_eq!(get_percentile_value(&values, 1000.0), Some(30));
    }

    #[test]
    fn value_sorted_uses_rounded_position() {
        // len=5 => indices 0..=4
        // p25 => position=1.0 => idx=1
        // p50 => position=2.0 => idx=2
        // p75 => position=3.0 => idx=3
        let values = [0, 1, 2, 3, 4];
        assert_eq!(get_percentile_value(&values, 25.0), Some(1));
        assert_eq!(get_percentile_value(&values, 50.0), Some(2));
        assert_eq!(get_percentile_value(&values, 75.0), Some(3));
    }

    #[test]
    fn value_f32_sorted_returns_zero_for_empty_input() {
        let values: [f32; 0] = [];
        assert_eq!(get_percentile_value_f32(&values, 50.0), 0.0);
    }

    #[test]
    fn get_percentile_index_matches_value_selection() {
        let values = [10, 20, 30, 40];
        let idx = get_percentile_index(values.len(), 50.0).unwrap();
        assert_eq!(values[idx], get_percentile_value(&values, 50.0).unwrap());
    }
}
