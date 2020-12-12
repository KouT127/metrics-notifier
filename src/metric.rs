#[derive(Debug, PartialEq)]
pub struct AggregatedMetrics {
    pub average: f64,
    pub maximum: f64,
    pub minimum: f64,
}

impl Default for AggregatedMetrics {
    fn default() -> Self {
        Self {
            average: 0.0,
            maximum: 0.0,
            minimum: 0.0,
        }
    }
}