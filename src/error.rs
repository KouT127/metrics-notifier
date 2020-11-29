use std::error::Error;

use serde::{Deserialize, Serialize};

use bigdecimal::BigDecimal;
use rusoto_cloudwatch::GetMetricStatisticsError;
use rusoto_core::RusotoError;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum MetricsClientError {
    NoneValue,
    ToPrimitive,
    GetMetricsError(RusotoError<GetMetricStatisticsError>),
}

impl Display for MetricsClientError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            MetricsClientError::NoneValue => write!(f, "Value is None"),
            MetricsClientError::ToPrimitive => {
                write!(f, "Failed to convert bigDecimal to primitive")
            }
            MetricsClientError::GetMetricsError(ref error) => std::fmt::Display::fmt(error, f),
        }
    }
}

impl Error for MetricsClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            MetricsClientError::GetMetricsError(ref error) => Some(error),
            _ => None,
        }
    }
}

impl From<RusotoError<GetMetricStatisticsError>> for MetricsClientError {
    fn from(e: RusotoError<GetMetricStatisticsError>) -> MetricsClientError {
        MetricsClientError::GetMetricsError(e)
    }
}
