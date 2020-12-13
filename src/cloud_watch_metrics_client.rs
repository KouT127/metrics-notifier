use crate::error::MetricsNotifierError;
use async_trait::async_trait;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use rusoto_cloudwatch::{CloudWatch, CloudWatchClient, Datapoint, GetMetricStatisticsInput};

use std::convert::TryFrom;
use std::ops::{Add, Div};
use crate::metric::AggregatedMetrics;
use crate::time_range::TimeRange;

const DEFAULT_STATISTICS: [&'static str; 3] = ["Average", "Minimum", "Maximum"];

pub struct CloudWatchMetricsClient {
    client: CloudWatchClient,
}

#[async_trait]
pub trait Aggregate {
    async fn aggregate_metrics(&self, time_range: &TimeRange) -> Result<AggregatedMetrics, MetricsNotifierError>;
}

#[async_trait]
impl Aggregate for CloudWatchMetricsClient {
    async fn aggregate_metrics(&self, time_range: &TimeRange) -> Result<AggregatedMetrics, MetricsNotifierError> {
        let metrics = self
            .client
            .get_metric_statistics(GetMetricStatisticsInput {
                start_time: time_range.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                end_time: time_range.end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                metric_name: "CPUUtilization".to_string(),
                namespace: "AWS/EC2".to_string(),
                period: 0,
                statistics: Some(
                    DEFAULT_STATISTICS
                        .iter()
                        .map(|statistic| statistic.to_string())
                        .collect(),
                ),
                ..Default::default()
            })
            .await?;
        self.aggregate_data_points(metrics.datapoints)
    }
}

impl CloudWatchMetricsClient {
    fn new_with_client(client: CloudWatchClient) -> Self {
        CloudWatchMetricsClient { client }
    }

    fn aggregate_data_points(
        &self,
        data_points: Option<Vec<Datapoint>>,
    ) -> Result<AggregatedMetrics, MetricsNotifierError> {
        let data_points = data_points.map_or(vec![], |points| points);
        if data_points.is_empty() {
            return Ok(AggregatedMetrics::default());
        }
        let mut total = BigDecimal::from(0);
        let mut minimum = 100.0f64;
        let mut maximum = 0.0f64;
        let length = u32::try_from(data_points.len())?;
        let count = BigDecimal::from(length);
        for data_point in data_points {
            let average = data_point
                .average
                .map(|average| {
                    BigDecimal::from_f64(average).map_or(BigDecimal::from(0), |average| average)
                })
                .ok_or(MetricsNotifierError::NoneValue)?;
            total = total.add(average);

            minimum = minimum.min(data_point.minimum.ok_or(MetricsNotifierError::NoneValue)?);
            maximum = maximum.max(data_point.maximum.ok_or(MetricsNotifierError::NoneValue)?);
        }

        let decimal_average = total.div(count);
        let average = decimal_average
            .to_f64()
            .ok_or(MetricsNotifierError::ToPrimitive)?;
        Ok(AggregatedMetrics {
            average,
            maximum,
            minimum,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::cloud_watch_metrics_client::{Aggregate, AggregatedMetrics, CloudWatchMetricsClient};
    use crate::error::MetricsNotifierError;
    use rusoto_cloudwatch::{CloudWatchClient, Datapoint};
    use rusoto_core::Region;
    use rusoto_mock::{
        MockCredentialsProvider, MockRequestDispatcher, MockResponseReader, ReadMockResponse,
    };
    use crate::time_range::TimeRange;
    use chrono::{DateTime, Utc};
    use std::str::FromStr;
    use std::convert::TryFrom;


    #[tokio::test]
    async fn test_aggregate_metrics() {
        let mock = CloudWatchClient::new_with(
            MockRequestDispatcher::default().with_body(&*MockResponseReader::read_response(
                "test_resources/valid",
                "get_metrics_data.xml",
            )),
            MockCredentialsProvider,
            Default::default(),
        );

        let beginning_of_month = DateTime::<Utc>::from_str("2019-01-12T00:00:00.0+00:00").unwrap();
        let range = TimeRange::try_from(beginning_of_month).unwrap();
        let client = CloudWatchMetricsClient::new_with_client(mock);
        let result = client.aggregate_metrics(&range).await;

        assert_eq!(
            result.unwrap(),
            AggregatedMetrics {
                average: 51.8,
                maximum: 99.0,
                minimum: 10.0,
            }
        );
    }

    #[tokio::test]
    async fn test_aggregate_metrics_error() {
        let mock = CloudWatchClient::new_with(
            MockRequestDispatcher::with_status(400).with_body(&*MockResponseReader::read_response(
                "test_resources/error",
                "get_metrics_data.xml",
            )),
            MockCredentialsProvider,
            Default::default(),
        );

        let beginning_of_month = DateTime::<Utc>::from_str("2019-01-12T00:00:00.0+00:00").unwrap();
        let range = TimeRange::try_from(beginning_of_month).unwrap();
        let client = CloudWatchMetricsClient::new_with_client(mock);
        let result = client.aggregate_metrics(&range).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aggregate_data_points() {
        let client = CloudWatchMetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
        let result = client.aggregate_data_points(Some(vec![
            Datapoint {
                average: Some(55.5),
                maximum: Some(91.0),
                minimum: Some(11.0),
                extended_statistics: None,
                sample_count: None,
                sum: None,
                timestamp: None,
                unit: None,
            },
            Datapoint {
                average: Some(28.8),
                maximum: Some(92.0),
                minimum: Some(13.0),
                extended_statistics: None,
                sample_count: None,
                sum: None,
                timestamp: None,
                unit: None,
            },
            Datapoint {
                average: Some(40.2),
                maximum: Some(93.0),
                minimum: Some(12.0),
                extended_statistics: None,
                sample_count: None,
                sum: None,
                timestamp: None,
                unit: None,
            },
            Datapoint {
                average: Some(51.3),
                maximum: Some(93.0),
                minimum: Some(12.0),
                extended_statistics: None,
                sample_count: None,
                sum: None,
                timestamp: None,
                unit: None,
            },
        ]));
        assert_eq!(
            AggregatedMetrics {
                average: 43.95,
                maximum: 93.0,
                minimum: 11.0,
            },
            result.unwrap()
        );
    }

    #[tokio::test]
    async fn test_aggregate_when_zero_value() {
        let client = CloudWatchMetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
        let result = client.aggregate_data_points(Some(vec![]));
        assert_eq!(
            AggregatedMetrics {
                average: 0.0,
                maximum: 0.0,
                minimum: 0.0,
            },
            result.unwrap()
        );
    }

    #[tokio::test]
    async fn test_dont_aggregate_when_no_value() {
        let client = CloudWatchMetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
        let result = client.aggregate_data_points(Some(vec![Datapoint {
            average: None,
            maximum: None,
            minimum: None,
            extended_statistics: None,
            sample_count: None,
            sum: None,
            timestamp: None,
            unit: None,
        }]));
        assert_eq!(result.err().unwrap(), MetricsNotifierError::NoneValue)
    }
}
