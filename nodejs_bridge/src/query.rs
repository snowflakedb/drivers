use crate::BRIDGE;
use crate::error::BridgeError;
use crate::query_status::QueryStatus;
use crate::session::Session;
use crate::validation_utils::require_valid_query_id;
use sf_core::rest::snowflake::QueryStatusResult;
use std::time::Duration;

// TODO: Similar logic to query.rs exists in every wrapper. Consider moving it to sf_core.
const WAIT_FOR_RESULT_RETRY_PATTERN: [u32; 7] = [1, 1, 2, 3, 4, 8, 10];
const WAIT_FOR_RESULT_NO_DATA_MAX_RETRY: u32 = 24;
const WAIT_FOR_RESULT_RETRY_INTERVAL: Duration = Duration::from_millis(500);

pub(crate) async fn get_status(
    session: &Session,
    query_id: &str,
) -> Result<QueryStatus, BridgeError> {
    let result = fetch_status_result(session, query_id).await?;
    Ok(QueryStatus::parse(&result.status_name))
}

pub(crate) async fn get_status_throw_if_error(
    session: &Session,
    query_id: &str,
) -> Result<QueryStatus, BridgeError> {
    let result = fetch_status_result(session, query_id).await?;
    let status = QueryStatus::parse(&result.status_name);
    if status.is_an_error() {
        return Err(BridgeError::QueryStatusFailed {
            query_id: query_id.to_string(),
            error_code: result.error_code,
            error_message: result.error_message,
        });
    }
    Ok(status)
}

pub(crate) async fn wait_for_result(
    session: &Session,
    query_id: String,
    retry_interval: Option<Duration>,
) -> Result<(), BridgeError> {
    let retry_interval = retry_interval.unwrap_or(WAIT_FOR_RESULT_RETRY_INTERVAL);
    let mut no_data_counter = 0u32;
    let mut retry_pattern_pos = 0usize;
    loop {
        let status = get_status_throw_if_error(session, &query_id).await?;
        if !status.is_still_running() {
            if status == QueryStatus::Success {
                return Ok(());
            }
            return Err(BridgeError::QueryIdNotSuccess {
                query_id,
                status: status.as_str().to_string(),
            });
        }

        tokio::time::sleep(retry_interval * WAIT_FOR_RESULT_RETRY_PATTERN[retry_pattern_pos]).await;

        if status == QueryStatus::NoData {
            no_data_counter += 1;
            if no_data_counter > WAIT_FOR_RESULT_NO_DATA_MAX_RETRY {
                return Err(BridgeError::QueryIdNoData(query_id));
            }
        }

        if retry_pattern_pos < WAIT_FOR_RESULT_RETRY_PATTERN.len() - 1 {
            retry_pattern_pos += 1;
        }
    }
}

async fn fetch_status_result(
    session: &Session,
    query_id: &str,
) -> Result<QueryStatusResult, BridgeError> {
    require_valid_query_id(query_id)?;
    let ready = session.ready().await?;
    BRIDGE
        .driver
        .connection_get_query_status(None, ready.connection(), query_id)
        .await
        .map_err(BridgeError::from)
}
