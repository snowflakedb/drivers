use crate::error::BridgeError;
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

pub(crate) fn is_valid_query_id(query_id: &str) -> bool {
    let bytes = query_id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, &b)| match i {
        8 | 13 | 18 | 23 => b == b'-',
        _ => b.is_ascii_hexdigit(),
    })
}

pub(crate) fn require_valid_query_id(query_id: &str) -> Result<(), BridgeError> {
    if is_valid_query_id(query_id) {
        Ok(())
    } else {
        Err(BridgeError::InvalidQueryId(query_id.to_string()))
    }
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

    #[test]
    fn query_id_accepts_hyphenated_hex_of_either_case() {
        assert!(require_valid_query_id("01c715df-0e1a-450a-000c-a913e79f70cb").is_ok());
        assert!(require_valid_query_id("12345678-1234-4123-A123-123456789012").is_ok());
        assert!(require_valid_query_id("00000000-0000-0000-0000-000000000000").is_ok());
    }

    #[test]
    fn query_id_rejects_malformed_values() {
        for query_id in [
            "invalidQueryId",
            "",
            "01c715df0e1a450a000ca913e79f70cb",
            "{01c715df-0e1a-450a-000c-a913e79f70cb}",
            "01c715df-0e1a-450a-000c-a913e79f70cb ",
        ] {
            assert!(
                matches!(
                    require_valid_query_id(query_id),
                    Err(BridgeError::InvalidQueryId(id)) if id == query_id
                ),
                "{query_id}"
            );
        }
    }
}
