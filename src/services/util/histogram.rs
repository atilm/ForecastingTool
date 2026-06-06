use thiserror::Error;

#[derive(Error, Debug)]
pub enum HistogramError {
    #[error("Empty data.")]
    EmptyData,
}

pub struct Histogram {
    pub min_value: f32,
    pub max_value: f32,
    pub bin_width: f32,
    pub bins: Vec<i32>,
}

impl Histogram {
    pub fn create(data: &[f32]) -> Result<Self, HistogramError> {
        if data.is_empty() {
            return Err(HistogramError::EmptyData);
        }

        let min_value = data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_value = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let bin_count = (data.len() as f32).sqrt().ceil() as usize;
        let bin_width = if (max_value - min_value).abs() < f32::EPSILON {
            1.0
        } else {
            (max_value - min_value) / bin_count as f32
        };

        let mut bins: Vec<i32> = vec![0; bin_count];
        for &value in data {
            let bin_index = ((value - min_value) / bin_width).floor() as usize;
            // Handle edge case where value is exactly max_value
            let clamped_index = bin_index.min(bin_count - 1);
            bins[clamped_index] += 1;
        }

        Ok(Self {
            min_value,
            max_value,
            bin_width,
            bins,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_empty_array_an_error_is_retunred() {
        let data: Vec<f32> = vec![];

        let result = Histogram::create(&data);

        assert!(result.is_err());
    }

    #[test]
    fn for_data_with_one_element_a_histogram_with_one_bin_is_returned() {
        let data: Vec<f32> = vec![5.0];

        let result = Histogram::create(&data);

        assert!(result.is_ok());
        let histogram = result.unwrap();
        assert_eq!(histogram.min_value, 5.0);
        assert_eq!(histogram.max_value, 5.0);
        assert_eq!(histogram.bin_width, 1.0);
        assert_eq!(histogram.bins.len(), 1);
        assert_eq!(histogram.bins[0], 1);
    }

    #[test]
    fn for_data_with_two_elements_a_histogram_with_two_bins_is_returned() {
        let data: Vec<f32> = vec![5.0, 10.0];

        let result = Histogram::create(&data);

        assert!(result.is_ok());
        let histogram = result.unwrap();
        assert_eq!(histogram.min_value, 5.0);
        assert_eq!(histogram.max_value, 10.0);
        assert_eq!(histogram.bin_width, 2.5);
        assert_eq!(histogram.bins.len(), 2);
        assert_eq!(histogram.bins[0], 1);
        assert_eq!(histogram.bins[1], 1);
    }

    #[test]
    fn bin_count_is_calculated_as_ceil_of_square_root_of_data_length() {
        let test_cases = vec![
            (vec![1.0], 1),
            (vec![1.0, 2.0], 2),
            (vec![1.0, 2.0, 3.0], 2),
            (vec![1.0, 2.0, 3.0, 4.0], 2),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0], 4),
        ];
        for (data, expected_bin_count) in test_cases {
            let result = Histogram::create(&data);
            assert!(result.is_ok());
            let histogram = result.unwrap();
            assert_eq!(histogram.bins.len(), expected_bin_count);
        }
    }

    #[test]
    fn bin_interval_borders_are_calculated_correctly() {
        let data: Vec<f32> = vec![0.0, 2.9999, 3.0, 4.0, 5.0, 5.9999, 6.0, 7.9999, 9.0];

        let result = Histogram::create(&data);

        assert!(result.is_ok());
        let histogram = result.unwrap();
        assert_eq!(histogram.min_value, 0.0);
        assert_eq!(histogram.max_value, 9.0);
        assert_eq!(histogram.bin_width, 3.0);

        assert_eq!(histogram.bins.len(), 3);
        assert_eq!(histogram.bins[0], 2); // 0.0, 2.9999
        assert_eq!(histogram.bins[1], 4); // 3.0, 4.0, 5.0, 5.9999
        assert_eq!(histogram.bins[2], 3); // 6.0, 7.9999, 9.0
    }
}
