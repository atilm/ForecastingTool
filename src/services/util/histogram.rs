use thiserror::Error;

#[derive(Error, Debug)]
pub enum HistogramError {
    #[error("Empty data.")]
    EmptyData,
}

pub struct HistogramBucket {
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub count: usize,
}

pub struct HistogramIter<'a> {
    histogram: &'a Histogram,
    index: usize,
}

pub struct HistogramIntoIter {
    min_value: f32,
    bin_width: f32,
    index: usize,
    bins: std::vec::IntoIter<i32>,
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

        // Bin count is the number of days in the data. Therefore the bin width is one day.
        let bin_count = (max_value - min_value).ceil().max(1.0) as usize;
        let bin_width = 1.0;

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

    pub fn iter(&self) -> HistogramIter<'_> {
        HistogramIter {
            histogram: self,
            index: 0,
        }
    }
}

impl Iterator for HistogramIter<'_> {
    type Item = HistogramBucket;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.histogram.bins.len() {
            return None;
        }

        let lower_bound = self.histogram.min_value + self.index as f32 * self.histogram.bin_width;
        let upper_bound = lower_bound + self.histogram.bin_width;
        let count = self.histogram.bins[self.index] as usize;
        self.index += 1;

        Some(HistogramBucket {
            lower_bound,
            upper_bound,
            count,
        })
    }
}

impl Iterator for HistogramIntoIter {
    type Item = HistogramBucket;

    fn next(&mut self) -> Option<Self::Item> {
        let count = self.bins.next()?;
        let lower_bound = self.min_value + self.index as f32 * self.bin_width;
        let upper_bound = lower_bound + self.bin_width;
        self.index += 1;

        Some(HistogramBucket {
            lower_bound,
            upper_bound,
            count: count as usize,
        })
    }
}

impl<'a> IntoIterator for &'a Histogram {
    type Item = HistogramBucket;
    type IntoIter = HistogramIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Histogram {
    type Item = HistogramBucket;
    type IntoIter = HistogramIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        HistogramIntoIter {
            min_value: self.min_value,
            bin_width: self.bin_width,
            index: 0,
            bins: self.bins.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6);
    }

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

    #[test]
    fn iterator_for_single_bin_returns_one_bucket_in_min_to_max_order() {
        let histogram = Histogram::create(&[5.0]).unwrap();

        let buckets: Vec<HistogramBucket> = histogram.iter().collect();

        assert_eq!(buckets.len(), 1);
        assert_f32_eq(buckets[0].lower_bound, 5.0);
        assert_f32_eq(buckets[0].upper_bound, 6.0);
        assert_eq!(buckets[0].count, 1);
    }

    #[test]
    fn iterator_includes_empty_buckets_between_populated_buckets() {
        let histogram = Histogram::create(&[0.0, 6.0, 20.0, 20.0, 21.0]).unwrap();

        let buckets: Vec<HistogramBucket> = histogram.iter().collect();

        assert_eq!(buckets.len(), 3);
        assert_f32_eq(buckets[0].lower_bound, 0.0);
        assert_f32_eq(buckets[0].upper_bound, 7.0);
        assert_eq!(buckets[0].count, 2);

        assert_f32_eq(buckets[1].lower_bound, 7.0);
        assert_f32_eq(buckets[1].upper_bound, 14.0);
        assert_eq!(buckets[1].count, 0);

        assert_f32_eq(buckets[2].lower_bound, 14.0);
        assert_f32_eq(buckets[2].upper_bound, 21.0);
        assert_eq!(buckets[2].count, 3);
    }

    #[test]
    fn iterator_non_trivial_example_returns_expected_bucket_ranges_and_counts() {
        let data = vec![1.0, 1.2, 2.8, 3.0, 4.1, 4.2, 5.9, 6.0, 7.7];
        let histogram = Histogram::create(&data).unwrap();

        let buckets: Vec<HistogramBucket> = histogram.into_iter().collect();

        assert_eq!(buckets.len(), 3);

        assert_f32_eq(buckets[0].lower_bound, 1.0);
        assert_f32_eq(buckets[0].upper_bound, 3.2333333);
        assert_eq!(buckets[0].count, 4);

        assert_f32_eq(buckets[1].lower_bound, 3.2333333);
        assert_f32_eq(buckets[1].upper_bound, 5.4666667);
        assert_eq!(buckets[1].count, 2);

        assert_f32_eq(buckets[2].lower_bound, 5.4666667);
        assert_f32_eq(buckets[2].upper_bound, 7.7000003);
        assert_eq!(buckets[2].count, 3);
    }
}
