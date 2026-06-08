use thiserror::Error;
use rand::Rng;

#[derive(Error, Debug, PartialEq)]
pub enum HistogramError {
    #[error("Empty data.")]
    EmptyData,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    pub min_value: f32,
    pub max_value: f32,
    pub bin_width: f32,
    pub bins: Vec<i32>,
    alias_table: (Vec<f32>, Vec<usize>), // (alias_prob, alias_bin)
}

impl Histogram {
    pub(crate) fn from_parts(min_value: f32, max_value: f32, bin_width: f32, bins: Vec<i32>) -> Self {
        let alias_table = Self::build_alias_table(&bins);
        Self {
            min_value,
            max_value,
            bin_width,
            bins,
            alias_table,
        }
    }

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

        let alias_table = Self::build_alias_table(&bins);

        Ok(Self {
            min_value,
            max_value,
            bin_width,
            bins,
            alias_table,
        })
    }

    /// Build the alias table using Vose's algorithm.
    /// See: https://www.keithschwarz.com/darts-dice-coins/
    /// Returns (alias_prob, alias_bin) where:
    /// - alias_prob[i] is the probability of using bin i (vs its alias)
    /// - alias_bin[i] is the index of the alias bin for bin i
    fn build_alias_table(bins: &[i32]) -> (Vec<f32>, Vec<usize>) {
        let n = bins.len();
        let total: i32 = bins.iter().sum();
        if total <= 0 {
            return (vec![1.0; n], (0..n).collect());
        }
        
        // Normalize to probabilities scaled by n
        let mut probs: Vec<f32> = bins.iter().map(|&b| (b as f32 * n as f32) / total as f32).collect();
        
        let mut alias_bin = vec![0; n];
        let mut alias_prob = vec![1.0; n];
        
        // Separate into overfull (prob > 1.0) and underfull (prob < 1.0) queues
        let mut overfull = Vec::new();
        let mut underfull = Vec::new();
        
        for i in 0..n {
            if probs[i] > 1.0 {
                overfull.push(i);
            } else {
                underfull.push(i);
            }
        }
        
        // Pair overfull with underfull
        while !overfull.is_empty() && !underfull.is_empty() {
            let poor = underfull.pop().unwrap();
            let rich = overfull.pop().unwrap();
            alias_prob[poor] = probs[poor].clamp(0.0, 1.0);
            
            alias_bin[poor] = rich;
            
            // Transfer excess probability from rich to poor's alias
            probs[rich] = probs[rich] - (1.0 - probs[poor]);
            
            if probs[rich] > 1.0 {
                overfull.push(rich);
            } else if probs[rich] < 1.0 {
                underfull.push(rich);
            }
        }

        for i in overfull {
            alias_prob[i] = 1.0;
            alias_bin[i] = i;
        }
        for i in underfull {
            alias_prob[i] = 1.0;
            alias_bin[i] = i;
        }
        
        (alias_prob, alias_bin)
    }

    /// Sample a random value from the probability distribution described by the histogram.
    /// 
    /// Uses Vose's Alias Method for O(1) sampling.
    pub fn sample<R: Rng>(&self, rng: &mut R) -> Result<f32, HistogramError> {
        let total: i32 = self.bins.iter().sum();
        if total == 0 {
            return Err(HistogramError::EmptyData);
        }

        let (alias_prob, alias_bin) = &self.alias_table;
        let n = self.bins.len();
        
        // Pick a random bin
        let i = rng.gen_range(0..n);
        
        // With probability alias_prob[i], use bin i; otherwise use alias
        let use_primary = rng.gen_range(0.0..1.0) < alias_prob[i];
        let bin_index = if use_primary { i } else { alias_bin[i] };
        
        // Map bin index to a value within the bin's range
        let lower = self.min_value + bin_index as f32 * self.bin_width;
        let upper = lower + self.bin_width;
        let value = rng.gen_range(lower..upper);
        
        Ok(value)
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
    use rand::SeedableRng;
    use rand::rngs::StdRng;

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
    fn for_data_with_two_elements_a_histogram_with_bin_width_1_is_returned() {
        let data: Vec<f32> = vec![5.0, 10.0];

        let result = Histogram::create(&data);

        assert!(result.is_ok());
        let histogram = result.unwrap();
        assert_eq!(histogram.min_value, 5.0);
        assert_eq!(histogram.max_value, 10.0);
        assert_eq!(histogram.bin_width, 1.0);
        assert_eq!(histogram.bins.len(), 5);
        assert_eq!(histogram.bins[0], 1);
        assert_eq!(histogram.bins[4], 1);
    }

    #[test]
    fn bin_count_is_calculated_from_range_width_with_one_day_bins() {
        let test_cases = vec![
            (vec![1.0], 1),
            (vec![1.0, 2.0], 1),
            (vec![1.0, 2.0, 3.0], 2),
            (vec![1.0, 2.0, 3.0, 4.0], 3),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0], 4),
            (vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 5),
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
        let data: Vec<f32> = vec![0.0, 0.9999, 3.0, 4.0, 5.0, 5.9999, 6.0, 7.9999, 9.0];

        let result = Histogram::create(&data);

        assert!(result.is_ok());
        let histogram = result.unwrap();
        assert_eq!(histogram.min_value, 0.0);
        assert_eq!(histogram.max_value, 9.0);
        assert_eq!(histogram.bin_width, 1.0);

        assert_eq!(histogram.bins.len(), 9);
        assert_eq!(histogram.bins[0], 2); // 0.0, 0.9999
        assert_eq!(histogram.bins[2], 0);
        assert_eq!(histogram.bins[3], 1); // 3.0
        assert_eq!(histogram.bins[4], 1); // 4.0
        assert_eq!(histogram.bins[5], 2); // 5.0, 5.9999
        assert_eq!(histogram.bins[6], 1); // 6.0
        assert_eq!(histogram.bins[7], 1); // 7.9999
        assert_eq!(histogram.bins[8], 1); // 9.0
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

        assert_eq!(buckets.len(), 21);
        assert_f32_eq(buckets[0].lower_bound, 0.0);
        assert_f32_eq(buckets[0].upper_bound, 1.0);
        assert_eq!(buckets[0].count, 1);

        assert_f32_eq(buckets[1].lower_bound, 1.0);
        assert_f32_eq(buckets[1].upper_bound, 2.0);
        assert_eq!(buckets[1].count, 0);

        assert_f32_eq(buckets[20].lower_bound, 20.0);
        assert_f32_eq(buckets[20].upper_bound, 21.0);
        assert_eq!(buckets[20].count, 3);
    }

    #[test]
    fn iterator_non_trivial_example_returns_expected_bucket_ranges_and_counts() {
        let data = vec![1.0, 1.2, 2.8, 3.0, 4.1, 4.2, 5.9, 6.0, 7.7];
        let histogram = Histogram::create(&data).unwrap();

        let buckets: Vec<HistogramBucket> = histogram.into_iter().collect();

        assert_eq!(buckets.len(), 7);
        assert_f32_eq(buckets[0].lower_bound, 1.0);
        assert_f32_eq(buckets[0].upper_bound, 2.0);
        assert_eq!(buckets[0].count, 2);
        assert_f32_eq(buckets[1].lower_bound, 2.0);
        assert_f32_eq(buckets[1].upper_bound, 3.0);
        assert_eq!(buckets[1].count, 1);
        assert_f32_eq(buckets[2].lower_bound, 3.0);
        assert_f32_eq(buckets[2].upper_bound, 4.0);
        assert_eq!(buckets[2].count, 1);
        assert_f32_eq(buckets[3].lower_bound, 4.0);
        assert_f32_eq(buckets[3].upper_bound, 5.0);
        assert_eq!(buckets[3].count, 2);
        assert_f32_eq(buckets[4].lower_bound, 5.0);
        assert_f32_eq(buckets[4].upper_bound, 6.0);
        assert_eq!(buckets[4].count, 1);
        assert_f32_eq(buckets[5].lower_bound, 6.0);
        assert_f32_eq(buckets[5].upper_bound, 7.0);
        assert_eq!(buckets[5].count, 1);
        assert_f32_eq(buckets[6].lower_bound, 7.0);
        assert_f32_eq(buckets[6].upper_bound, 8.0);
        assert_eq!(buckets[6].count, 1);
    }

    #[test]
    fn sample_from_single_bin_histogram_returns_value_in_bin_range() {
        let histogram = Histogram::create(&[5.0]).unwrap();
        let mut rng = StdRng::seed_from_u64(42);

        let sample = histogram.sample(&mut rng).unwrap();

        assert!(sample >= 5.0 && sample < 6.0);
    }

    #[test]
    fn sample_from_empty_histogram_returns_error() {
        let histogram = Histogram::create(&[1.0, 2.0]).unwrap();
        let mut rng = StdRng::seed_from_u64(42);

        // Manually create a histogram with all-zero bins to test the error case
        // (This is a bit of a hack since we can't directly create one through the API)
        let mut hist_zero = histogram;
        hist_zero.bins.iter_mut().for_each(|b| *b = 0);

        let result = hist_zero.sample(&mut rng);

        assert!(result.is_err());
    }

    #[test]
    fn sample_generates_values_within_histogram_range() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let histogram = Histogram::create(&data).unwrap();
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..100 {
            let sample = histogram.sample(&mut rng).unwrap();
            assert!(sample >= histogram.min_value && sample < histogram.max_value + histogram.bin_width);
        }
    }

    #[test]
    fn sample_distribution_roughly_matches_input_histogram() {
        let data = vec![1.0, 1.1, 1.2, 5.0, 5.1, 6.0, 6.1, 6.2, 6.3];
        let histogram = Histogram::create(&data).unwrap();
        let mut rng = StdRng::seed_from_u64(42);

        // Sample many times and count distribution
        let samples: Vec<f32> = (0..10000)
            .map(|_| histogram.sample(&mut rng).unwrap())
            .collect();

        // Check that samples follow roughly the right distribution
        // Low bin (1.0-2.33): should have ~33% (3 out of 9 values)
        // Mid bin (2.33-4.67): should have ~0% (0 out of 9 values)
        // High bin (4.67-6.3): should have ~67% (6 out of 9 values)
        
        let low_bin_count = samples.iter().filter(|&&s| s < 2.33).count();
        let high_bin_count = samples.iter().filter(|&&s| s >= 4.67).count();

        let low_ratio = low_bin_count as f32 / samples.len() as f32;
        let high_ratio = high_bin_count as f32 / samples.len() as f32;

        // Allow 10% tolerance due to randomness
        assert!((low_ratio - 0.333).abs() < 0.1, "Low bin ratio {}", low_ratio);
        assert!((high_ratio - 0.667).abs() < 0.1, "High bin ratio {}", high_ratio);
    }
}
