/// Percentile helpers for already-sorted slices.
///
/// - Empty input => `None` (or `0.0` for the f32 convenience wrapper).
/// - `percentile <= 0` => first element.
/// - `percentile >= 100` => last element.
/// - Otherwise we compute a position within `[0, len-1]` and round to the
///   nearest index.

/// Returns the percentile value from a slice that is already sorted in
/// ascending order.
pub fn value_sorted<T: Copy>(sorted_values: &[T], percentile: f64) -> Option<T> {
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

/// Convenience wrapper for `f32` results.
pub fn value_f32_sorted(sorted_values: &[f32], percentile: f64) -> f32 {
    value_sorted(sorted_values, percentile).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_sorted_returns_none_for_empty_input() {
        let values: [i32; 0] = [];
        assert_eq!(value_sorted(&values, 50.0), None);
    }

    #[test]
    fn value_sorted_clamps_to_first_and_last() {
        let values = [10, 20, 30];
        assert_eq!(value_sorted(&values, -1.0), Some(10));
        assert_eq!(value_sorted(&values, 0.0), Some(10));
        assert_eq!(value_sorted(&values, 100.0), Some(30));
        assert_eq!(value_sorted(&values, 1000.0), Some(30));
    }

    #[test]
    fn value_sorted_uses_rounded_position() {
        // len=5 => indices 0..=4
        // p25 => position=1.0 => idx=1
        // p50 => position=2.0 => idx=2
        // p75 => position=3.0 => idx=3
        let values = [0, 1, 2, 3, 4];
        assert_eq!(value_sorted(&values, 25.0), Some(1));
        assert_eq!(value_sorted(&values, 50.0), Some(2));
        assert_eq!(value_sorted(&values, 75.0), Some(3));
    }

    #[test]
    fn value_f32_sorted_returns_zero_for_empty_input() {
        let values: [f32; 0] = [];
        assert_eq!(value_f32_sorted(&values, 50.0), 0.0);
    }
}
