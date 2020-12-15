use std::error::Error;

use rusoto_cloudwatch::GetMetricStatisticsError;
use rusoto_core::RusotoError;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::num::TryFromIntError;
use rusoto_ec2::DescribeInstancesError;

#[derive(Debug, PartialEq)]
pub enum MetricsNotifierError {
    NoneValue,
    ToPrimitive,
    TryFromIntError,
    GetMetricsError(RusotoError<GetMetricStatisticsError>),
    DescribeInstancesError(RusotoError<DescribeInstancesError>),
}

impl Display for MetricsNotifierError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            MetricsNotifierError::NoneValue => write!(f, "Value is None"),
            MetricsNotifierError::ToPrimitive => {
                write!(f, "Failed to convert bigDecimal to primitive")
            }
            MetricsNotifierError::TryFromIntError => write!(f, "Failed to convert int"),
            MetricsNotifierError::GetMetricsError(ref error) => std::fmt::Display::fmt(error, f),
            MetricsNotifierError::DescribeInstancesError(ref error) => std::fmt::Display::fmt(error, f),
        }
    }
}

impl Error for MetricsNotifierError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            MetricsNotifierError::GetMetricsError(ref error) => Some(error),
            _ => None,
        }
    }
}

impl From<TryFromIntError> for MetricsNotifierError {
    fn from(_: TryFromIntError) -> MetricsNotifierError {
        MetricsNotifierError::TryFromIntError
    }
}

impl From<RusotoError<GetMetricStatisticsError>> for MetricsNotifierError {
    fn from(e: RusotoError<GetMetricStatisticsError>) -> MetricsNotifierError {
        MetricsNotifierError::GetMetricsError(e)
    }
}

impl From<RusotoError<DescribeInstancesError>> for MetricsNotifierError {
    fn from(e: RusotoError<DescribeInstancesError>) -> MetricsNotifierError {
        MetricsNotifierError::DescribeInstancesError(e)
    }
}