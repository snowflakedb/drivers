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
}

impl FromStr for QueryStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "RUNNING" => Ok(Self::Running),
            "ABORTING" => Ok(Self::Aborting),
            "SUCCESS" => Ok(Self::Success),
            "FAILED_WITH_ERROR" => Ok(Self::FailedWithError),
            "ABORTED" => Ok(Self::Aborted),
            "QUEUED" => Ok(Self::Queued),
            "FAILED_WITH_INCIDENT" => Ok(Self::FailedWithIncident),
            "DISCONNECTED" => Ok(Self::Disconnected),
            "RESUMING_WAREHOUSE" => Ok(Self::ResumingWarehouse),
            "QUEUED_REPARING_WAREHOUSE" => Ok(Self::QueuedReparingWarehouse),
            "RESTARTED" => Ok(Self::Restarted),
            "BLOCKED" => Ok(Self::Blocked),
            "NO_DATA" => Ok(Self::NoData),
            _ => Err(()),
        }
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
