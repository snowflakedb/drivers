use napi_derive::napi;
use std::str::FromStr;

#[napi(string_enum = "UPPER_SNAKE")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryStatus {
    Running,
    Aborting,
    Success,
    FailedWithError,
    Aborted,
    Queued,
    FailedWithIncident,
    Disconnected,
    ResumingWarehouse,
    QueuedReparingWarehouse,
    Restarted,
    Blocked,
    NoData,
}

impl QueryStatus {
    const ALL: [(Self, &'static str); 13] = [
        (Self::Running, "RUNNING"),
        (Self::Aborting, "ABORTING"),
        (Self::Success, "SUCCESS"),
        (Self::FailedWithError, "FAILED_WITH_ERROR"),
        (Self::Aborted, "ABORTED"),
        (Self::Queued, "QUEUED"),
        (Self::FailedWithIncident, "FAILED_WITH_INCIDENT"),
        (Self::Disconnected, "DISCONNECTED"),
        (Self::ResumingWarehouse, "RESUMING_WAREHOUSE"),
        (Self::QueuedReparingWarehouse, "QUEUED_REPARING_WAREHOUSE"),
        (Self::Restarted, "RESTARTED"),
        (Self::Blocked, "BLOCKED"),
        (Self::NoData, "NO_DATA"),
    ];

    pub(crate) fn parse(status_name: &str) -> Self {
        Self::from_str(status_name).unwrap_or(Self::NoData)
    }

    pub(crate) fn is_an_error(self) -> bool {
        matches!(
            self,
            Self::Aborting
                | Self::FailedWithError
                | Self::Aborted
                | Self::FailedWithIncident
                | Self::Disconnected
        )
    }

    pub(crate) fn is_still_running(self) -> bool {
        matches!(
            self,
            Self::Running
                | Self::Queued
                | Self::ResumingWarehouse
                | Self::QueuedReparingWarehouse
                | Self::Blocked
                | Self::NoData
        )
    }

    pub(crate) fn as_str(self) -> &'static str {
        for (status, name) in Self::ALL {
            if status == self {
                return name;
            }
        }
        unreachable!("QueryStatus::ALL lists every variant")
    }
}

impl FromStr for QueryStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let upper = s.to_ascii_uppercase();
        for (status, name) in Self::ALL {
            if name == upper {
                return Ok(status);
            }
        }
        Err(())
    }
}

#[napi]
pub fn is_an_error(status: QueryStatus) -> bool {
    status.is_an_error()
}

#[napi]
pub fn is_still_running(status: QueryStatus) -> bool {
    status.is_still_running()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_status_parse_roundtrips_as_str_and_is_case_insensitive() {
        for (status, name) in QueryStatus::ALL {
            assert_eq!(status.as_str(), name);
            assert_eq!(QueryStatus::from_str(name), Ok(status));
        }
        assert_eq!(QueryStatus::parse("restarted"), QueryStatus::Restarted);
        assert_eq!(QueryStatus::parse("not-a-status"), QueryStatus::NoData);
    }
}
