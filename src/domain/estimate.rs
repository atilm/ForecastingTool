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
pub enum Estimate {
    StoryPoint(StoryPointEstimate),
    ThreePoint(ThreePointEstimate),
    Reference(ReferenceEstimate),
}