use crate::error::MetricsClientError;
use async_trait::async_trait;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use rusoto_cloudwatch::{CloudWatch, CloudWatchClient, Datapoint, GetMetricStatisticsInput};

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use std::convert::TryFrom;
use std::ops::{Add, Div};

const DEFAULT_STATISTICS: [&'static str; 3] = ["Average", "Minimum", "Maximum"];

#[derive(Debug, PartialEq)]
struct TimeRange {
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
}

impl TryFrom<DateTime<Utc>> for TimeRange {
    type Error = MetricsClientError;

    fn try_from(date_time: DateTime<Utc>) -> Result<Self, Self::Error> {
        let tokyo = FixedOffset::east(9 * 3600);
        let now: DateTime<FixedOffset> = date_time.with_timezone(&tokyo);
        let start = tokyo
            .from_local_datetime(
                &chrono::NaiveDate::from_ymd(now.year(), now.month(), 1).and_hms(0, 0, 0),
            )
            .single()
            .ok_or_else(|| MetricsClientError::NoneValue)?;

        let end = tokyo
            .from_local_datetime(
                &chrono::NaiveDate::from_ymd(
                    now.year(),
                    now.month(),
                    Self::last_day_of_month(now.year(), now.month()),
                )
                .and_hms(23, 59, 59),
            )
            .single()
            .ok_or_else(|| MetricsClientError::NoneValue)?;

        Ok(TimeRange {
            start: start.with_timezone(&Utc),
            end: end.with_timezone(&Utc),
        })
    }
}

impl TimeRange {
    fn last_day_of_month(year: i32, month: u32) -> u32 {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap_or(NaiveDate::from_ymd(year + 1, 1, 1))
            .pred()
            .day()
    }
}

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

pub struct MetricsClient {
    client: CloudWatchClient,
}

#[async_trait]
pub trait Aggregate {
    async fn aggregate_metrics(self) -> Result<AggregatedMetrics, MetricsClientError>;
}

#[async_trait]
impl Aggregate for MetricsClient {
    async fn aggregate_metrics(self) -> Result<AggregatedMetrics, MetricsClientError> {
        let metrics = self
            .client
            .get_metric_statistics(GetMetricStatisticsInput {
                start_time: "".to_string(),
                end_time: "".to_string(),
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

impl MetricsClient {
    fn new_with_client(client: CloudWatchClient) -> Self {
        MetricsClient { client }
    }

    fn aggregate_data_points(
        self,
        data_points: Option<Vec<Datapoint>>,
    ) -> Result<AggregatedMetrics, MetricsClientError> {
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
                .ok_or(MetricsClientError::NoneValue)?;
            total = total.add(average);

            minimum = minimum.min(data_point.minimum.ok_or(MetricsClientError::NoneValue)?);
            maximum = maximum.max(data_point.maximum.ok_or(MetricsClientError::NoneValue)?);
        }

        let decimal_average = total.div(count);
        let average = decimal_average
            .to_f64()
            .ok_or(MetricsClientError::ToPrimitive)?;
        Ok(AggregatedMetrics {
            average,
            maximum,
            minimum,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::client::{Aggregate, AggregatedMetrics, MetricsClient, TimeRange};
    use crate::error::MetricsClientError;
    use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
    use rusoto_cloudwatch::{CloudWatchClient, Datapoint};
    use rusoto_core::Region;
    use rusoto_mock::{
        MockCredentialsProvider, MockRequestDispatcher, MockResponseReader, ReadMockResponse,
    };
    use std::convert::TryFrom;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_try_from() {
        let beginning_of_month = chrono::NaiveDate::from_ymd(2020, 12, 1).and_hms(0, 0, 0);
        let datetime = DateTime::<Utc>::from_str("2020-12-01T15:00:00.0+00:00").unwrap();

        let time_range = TimeRange::try_from(datetime);
        assert_eq!(
            time_range.unwrap(),
            TimeRange {
                start: Utc::from_utc_datetime(
                    &Utc {},
                    &chrono::NaiveDate::from_ymd(2020, 11, 30).and_hms(15, 0, 0),
                ),
                end: Utc::from_utc_datetime(
                    &Utc {},
                    &chrono::NaiveDate::from_ymd(2020, 12, 31).and_hms(14, 59, 59),
                ),
            }
        );
    }

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

        let client = MetricsClient::new_with_client(mock);
        let result = client.aggregate_metrics().await;

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

        let client = MetricsClient::new_with_client(mock);
        let result = client.aggregate_metrics().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aggregate_data_points() {
        let client = MetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
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
        let client = MetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
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
        let client = MetricsClient::new_with_client(CloudWatchClient::new(Region::ApNortheast3));
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
        assert_eq!(result.err().unwrap(), MetricsClientError::NoneValue)
    }
}
