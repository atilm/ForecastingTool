const ESTIMATES: [f32; 13] = [
    0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 20.0, 40.0, 100.0, 200.0, 400.0, 800.0,
];

pub(crate) fn bounds(value: f32) -> (f32, f32) {
    // let bounds_estimates = &ESTIMATES[..12];
    let bounds_estimates = &ESTIMATES;
    let lower_than_one = (bounds_estimates[0] + bounds_estimates[1]) / 2.0;
    let greater_than_one = (bounds_estimates[1] + bounds_estimates[2]) / 2.0;

    if value <= lower_than_one {
        return (bounds_estimates[0], bounds_estimates[2]);
    }

    if value <= greater_than_one {
        return (bounds_estimates[0], bounds_estimates[3]);
    }

    for window in bounds_estimates.windows(5) {
        let lower_limit = (window[1] + window[2]) / 2.0;
        let upper_limit = (window[2] + window[3]) / 2.0;
        if value > lower_limit && value <= upper_limit {
            return (window[0], window[4]);
        }
    }

    let lower_than_400 = (bounds_estimates[10] + bounds_estimates[11]) / 2.0;
    let greater_than_400 = (bounds_estimates[11] + bounds_estimates[12]) / 2.0;
    if value > lower_than_400 && value <= greater_than_400 {
        return (bounds_estimates[9], bounds_estimates[12]);
    }

    if value > greater_than_400 && value <= bounds_estimates[12] {
        return (bounds_estimates[10], bounds_estimates[12]);
    }

    (value, value)
}

pub(crate) fn previous_value(value: f32) -> f32 {
    ESTIMATES
        .windows(2)
        .find_map(|window| (value <= window[1]).then_some(window[0]))
        .unwrap_or(value)
        .max(1.0)
}

pub(crate) fn largest_value_at_most(value: f32) -> f32 {
    ESTIMATES
        .iter()
        .copied()
        .filter(|estimate| *estimate <= value)
        .last()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_span_two_fibonacci_steps() {
        let test_cases = [
            (5.0, (2.0, 13.0)),
            (4.01, (2.0, 13.0)),
            (6.49, (2.0, 13.0)),
            (-1.0, (0.0, 2.0)),
            (0.49, (0.0, 2.0)),
            (0.51, (0.0, 3.0)),
            (1.49, (0.0, 3.0)),
            (1.51, (0.0, 5.0)),
            (30.01, (13.0, 200.0)),
            (200.0, (40.0, 800.0)),
            (300.01, (100.0, 800.0)),
            (401.0, (100.0, 800.0)),
            (801.0, (801.0, 801.0)),
        ];

        for (value, expected) in test_cases {
            assert_eq!(bounds(value), expected, "unexpected bounds for {value}");
        }
    }

    #[test]
    fn previous_value_returns_prior_estimate_and_floors_at_one() {
        let test_cases = [
            (1.0, 1.0),
            (2.0, 1.0),
            (8.0, 5.0),
            (8.1, 8.0),
            (13.0, 8.0),
            (20.0, 13.0),
            (801.0, 801.0),
        ];

        for (value, expected) in test_cases {
            assert_eq!(
                previous_value(value),
                expected,
                "unexpected previous value for {value}"
            );
        }
    }

    #[test]
    fn largest_value_at_most_uses_configured_scale() {
        assert_eq!(largest_value_at_most(0.9), 0.0);
        assert_eq!(largest_value_at_most(5.0), 5.0);
        assert_eq!(largest_value_at_most(7.9), 5.0);
        assert_eq!(largest_value_at_most(900.0), 800.0);
    }
}
