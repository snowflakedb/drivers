use napi_derive::napi;

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

#[napi]
pub fn is_an_error(status: QueryStatus) -> bool {
    status.is_an_error()
}

#[napi]
pub fn is_still_running(status: QueryStatus) -> bool {
    status.is_still_running()
}
