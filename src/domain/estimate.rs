
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
pub enum Estimate {
    StoryPoint(StoryPointEstimate),
    ThreePoint(ThreePointEstimate),
}