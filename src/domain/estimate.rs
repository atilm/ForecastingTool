use crate::services::util::histogram::Histogram;

#[derive(Debug, Clone, PartialEq)]
pub struct StoryPointEstimate {
    pub estimate: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreePointEstimate {
    pub optimistic: Option<f32>,
    pub most_likely: Option<f32>,
    pub pessimistic: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
/// Links to a simulation report file whose 0, 50 and 100
/// percentiles should be used as the estimate.
pub struct ReferenceEstimate {
    pub report_file_path: String,
    pub cached_estimate: Option<ThreePointEstimate>,
}

#[derive(Debug, Clone, PartialEq)]
/// Links to a histogram yaml file whose distribution should be sampled
/// directly during project simulation.
pub struct HistogramReferenceEstimate {
    pub histogram_file_path: String,
    pub cached_histogram: Option<Histogram>,
    pub elapsed_days: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Estimate {
    StoryPoint(StoryPointEstimate),
    ThreePoint(ThreePointEstimate),
    Reference(ReferenceEstimate),
    HistogramReference(HistogramReferenceEstimate),
    Milestone,
}
