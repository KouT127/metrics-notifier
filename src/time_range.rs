use std::convert::TryFrom;
use chrono::{DateTime, Utc, FixedOffset, TimeZone, Datelike, NaiveDate};
use crate::error::MetricsNotifierError;

#[derive(Debug, PartialEq)]
pub struct TimeRange {
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
}

impl TryFrom<DateTime<Utc>> for TimeRange {
    type Error = MetricsNotifierError;

    fn try_from(date_time: DateTime<Utc>) -> Result<Self, Self::Error> {
        let tokyo = FixedOffset::east(9 * 3600);
        let now: DateTime<FixedOffset> = date_time.with_timezone(&tokyo);
        let start = tokyo
            .from_local_datetime(
                &chrono::NaiveDate::from_ymd(now.year(), now.month(), 1).and_hms(0, 0, 0),
            )
            .single()
            .ok_or_else(|| MetricsNotifierError::NoneValue)?;

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
            .ok_or_else(|| MetricsNotifierError::NoneValue)?;

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

#[cfg(test)]
mod tests {
    use crate::time_range::TimeRange;
    use chrono::{Utc, TimeZone, DateTime};
    use std::convert::TryFrom;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_try_from() {
        let beginning_of_month = DateTime::<Utc>::from_str("2020-12-01T15:00:00.0+00:00").unwrap();

        let time_range = TimeRange::try_from(beginning_of_month);
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
}