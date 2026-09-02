//! Exhaustive walk of [`RestError`] for vendor / SQLSTATE / query id / request id
//!
//! Adding a variant is a compile error until it is classified,
//! as there is no wildcard that silently drops those fields.

use super::sql_state::sql_state_from_code;
use super::{
    CREDENTIAL_REJECTION_GS_CODES, GS_CODE_UNAVAILABLE, QueryIds, RestError, SESSION_TOKEN_EXPIRED,
    SQLSTATE_AUTHORIZATION_FAILURE, SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED,
    SQLSTATE_TIMEOUT_EXPIRED,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SnowflakeErrorContext {
    pub vendor_code: Option<i32>,
    pub sql_state: Option<String>,
    pub query_id: Option<String>,
    pub request_id: Option<String>,
}

impl SnowflakeErrorContext {
    pub(crate) fn gs_connection_not_established(vendor_code: i32) -> Self {
        Self {
            vendor_code: Some(vendor_code),
            sql_state: Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED.to_string()),
            ..Default::default()
        }
    }

    fn with_sql_state_fallback(mut self) -> Self {
        if self.sql_state.is_none() {
            self.sql_state = self
                .vendor_code
                .and_then(sql_state_from_code)
                .map(str::to_owned);
        }
        self
    }

    fn with_ids(mut self, ids: &QueryIds) -> Self {
        self.query_id = ids.query_id.clone();
        self.request_id = ids.request_id.map(|id| id.to_string());
        self
    }
}

impl RestError {
    /// Vendor code, SQLSTATE, query id, and request id for a [`RestError`].
    ///
    /// SQLSTATE resolution order (first hit wins):
    ///   1. The `sqlState` the server included in its response (verbatim).
    ///   2. For login failures, [`SQLSTATE_AUTHORIZATION_FAILURE`] ("28000")
    ///      when the code is a known credential rejection
    ///      ([`CREDENTIAL_REJECTION_GS_CODES`]);
    ///      [`SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED`] ("08001") when the
    ///      login failure is reauth-shaped; otherwise left unset so callers
    ///      fall back to their own default — legacy ODBC and Python disagree
    ///      on that default (28000 vs 08001), so it isn't resolved centrally.
    ///   3. Token-expiry and session-refresh GS failures:
    ///      [`SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED`] ("08001") via
    ///      [`SnowflakeErrorContext::gs_connection_not_established`]. Covers session expired
    ///      (`390112`), session-refresh / token-request failures with a GS
    ///      code (Python `_renew_session` always sets `08001`), and
    ///      master-token terminal. Without this, ODBC's
    ///      `ErrorKind::AuthenticationError` fallback would report `28000`.
    ///   4. `sql_state_from_code` lookup against the numeric Snowflake error
    ///      code, which covers paths (async-poll, query-monitoring) that drop
    ///      `sqlState` on the wire but keep the error code. Login and
    ///      master-token codes are deliberately not in that table.
    ///
    /// A code of [`GS_CODE_UNAVAILABLE`] (`-1`) means the server omitted or
    /// sent a non-numeric code; no vendor_code is surfaced. Message text is
    /// never inspected — classification belongs to the server.
    pub(crate) fn snowflake_context(&self) -> SnowflakeErrorContext {
        let snowflake_ctx = match self {
            RestError::QueryFailed {
                code,
                sql_state,
                ids,
                ..
            } => SnowflakeErrorContext {
                vendor_code: *code,
                sql_state: sql_state.clone(),
                ..Default::default()
            }
            .with_ids(ids),
            RestError::LoginError {
                code,
                reauthentication_required,
                ..
            } => {
                if *code == GS_CODE_UNAVAILABLE {
                    SnowflakeErrorContext::default()
                } else {
                    let sql_state = if CREDENTIAL_REJECTION_GS_CODES.contains(code) {
                        Some(SQLSTATE_AUTHORIZATION_FAILURE.to_string())
                    } else if *reauthentication_required {
                        Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED.to_string())
                    } else {
                        None
                    };
                    SnowflakeErrorContext {
                        vendor_code: Some(*code),
                        sql_state,
                        ..Default::default()
                    }
                }
            }
            RestError::SessionExpired { .. } => {
                SnowflakeErrorContext::gs_connection_not_established(SESSION_TOKEN_EXPIRED)
            }
            RestError::SessionRefreshFailed { code, .. }
            | RestError::TokenRequestFailed { code, .. } => {
                if *code == GS_CODE_UNAVAILABLE {
                    SnowflakeErrorContext::default()
                } else {
                    SnowflakeErrorContext::gs_connection_not_established(*code)
                }
            }
            RestError::OperationTimeout { ids, .. } => SnowflakeErrorContext {
                sql_state: Some(SQLSTATE_TIMEOUT_EXPIRED.to_string()),
                ..Default::default()
            }
            .with_ids(ids),
            RestError::MasterTokenTerminal { code, .. } => {
                SnowflakeErrorContext::gs_connection_not_established(*code)
            }
            RestError::HttpRetry { ids, .. }
            | RestError::AsyncPollResultNotFound { ids, .. }
            | RestError::MissingResultUrl { ids, .. }
            | RestError::MissingQueryId { ids, .. } => {
                SnowflakeErrorContext::default().with_ids(ids)
            }
            // no wildcard - explicit empty arms
            RestError::Authentication { .. }
            | RestError::NativeOkta { .. }
            | RestError::ExternalBrowser { .. }
            | RestError::OAuthFlow { .. }
            | RestError::WorkloadIdentityAttestation { .. }
            | RestError::InvalidSnowflakeResponse { .. }
            | RestError::Communication { .. }
            | RestError::RequestConstruction { .. }
            | RestError::CrlValidation { .. }
            | RestError::UrlJoin { .. }
            | RestError::SessionRefresh { .. }
            | RestError::TokenRequestHttp { .. }
            | RestError::Heartbeat { .. }
            | RestError::MissingResponseField { .. }
            | RestError::Logout { .. }
            | RestError::InvalidUrl { .. }
            | RestError::PayloadEncode { .. } => SnowflakeErrorContext::default(),
        };
        snowflake_ctx.with_sql_state_fallback()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::Location;

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    #[test]
    fn query_failed_passes_through_ids_and_vendor() {
        let request_id = uuid::Uuid::new_v4();
        let err = RestError::QueryFailed {
            message: "boom".to_owned(),
            code: Some(1003),
            sql_state: Some("42000".to_owned()),
            ids: QueryIds {
                request_id: Some(request_id),
                query_id: Some("01abc".to_owned()),
            },
            location: loc(),
            query_context: None,
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext {
                vendor_code: Some(1003),
                sql_state: Some("42000".to_owned()),
                query_id: Some("01abc".to_owned()),
                request_id: Some(request_id.to_string()),
            }
        );
    }

    #[test]
    fn query_failed_fills_sql_state_from_code_lookup() {
        let err = RestError::QueryFailed {
            message: "boom".to_owned(),
            code: Some(100038),
            sql_state: None,
            ids: QueryIds::default(),
            location: loc(),
            query_context: None,
        };
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.vendor_code, Some(100038));
        assert_eq!(snowflake_ctx.sql_state.as_deref(), Some("22003"));
    }

    #[test]
    fn login_credential_rejection_gets_authorization_sql_state() {
        let err = RestError::LoginError {
            message: "bad password".to_owned(),
            code: 390100,
            reauthentication_required: false,
            location: loc(),
        };
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.vendor_code, Some(390100));
        assert_eq!(snowflake_ctx.sql_state.as_deref(), Some("28000"));
        assert_eq!(snowflake_ctx.query_id, None);
        assert_eq!(snowflake_ctx.request_id, None);
    }

    #[test]
    fn login_reauth_gets_connection_sql_state() {
        let err = RestError::LoginError {
            message: "id token expired".to_owned(),
            code: 390195,
            reauthentication_required: true,
            location: loc(),
        };
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.vendor_code, Some(390195));
        assert_eq!(snowflake_ctx.sql_state.as_deref(), Some("08001"));
    }

    #[test]
    fn login_non_rejection_non_reauth_has_no_sql_state() {
        let err = RestError::LoginError {
            message: "session expired".to_owned(),
            code: 390111,
            reauthentication_required: false,
            location: loc(),
        };
        let snowflake_ctx = err.snowflake_context();
        assert_eq!(snowflake_ctx.vendor_code, Some(390111));
        assert_eq!(snowflake_ctx.sql_state, None);
    }

    #[test]
    fn session_expired_surfaces_390112_and_08001() {
        let err = RestError::SessionExpired { location: loc() };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext::gs_connection_not_established(SESSION_TOKEN_EXPIRED)
        );
    }

    #[test]
    fn session_refresh_failed_passes_through_vendor_code() {
        let err = RestError::SessionRefreshFailed {
            message: "expired".to_owned(),
            code: 390111,
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext::gs_connection_not_established(390111)
        );
    }

    #[test]
    fn token_request_failed_passes_through_vendor_code() {
        let err = RestError::TokenRequestFailed {
            operation: "RENEW".to_owned(),
            message: "expired".to_owned(),
            code: 390111,
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext::gs_connection_not_established(390111)
        );
    }

    #[test]
    fn token_request_failed_unavailable_code_stays_empty() {
        let err = RestError::TokenRequestFailed {
            operation: "RENEW".to_owned(),
            message: "expired".to_owned(),
            code: GS_CODE_UNAVAILABLE,
            location: loc(),
        };
        assert_eq!(err.snowflake_context(), SnowflakeErrorContext::default());
    }

    #[test]
    fn session_refresh_failed_unavailable_code_stays_empty() {
        let err = RestError::SessionRefreshFailed {
            message: "expired".to_owned(),
            code: GS_CODE_UNAVAILABLE,
            location: loc(),
        };
        assert_eq!(err.snowflake_context(), SnowflakeErrorContext::default());
    }

    #[test]
    fn query_http_retry_passes_through_request_id() {
        let request_id = uuid::Uuid::new_v4();
        let err = RestError::HttpRetry {
            context: "query request",
            ids: QueryIds {
                request_id: Some(request_id),
                query_id: None,
            },
            source: crate::http::retry::HttpError::MaxAttempts {
                attempts: 3,
                last_status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                location: loc(),
            },
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext {
                vendor_code: None,
                sql_state: None,
                query_id: None,
                request_id: Some(request_id.to_string()),
            }
        );
    }

    #[test]
    fn login_http_retry_has_no_ids() {
        let err = RestError::HttpRetry {
            context: "login request",
            ids: QueryIds::default(),
            source: crate::http::retry::HttpError::MaxAttempts {
                attempts: 3,
                last_status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                location: loc(),
            },
            location: loc(),
        };
        assert_eq!(err.snowflake_context(), SnowflakeErrorContext::default());
    }

    #[test]
    fn poll_protocol_errors_pass_through_ids() {
        let request_id = uuid::Uuid::new_v4();
        let ids = QueryIds {
            request_id: Some(request_id),
            query_id: Some("01abc".to_owned()),
        };
        for err in [
            RestError::AsyncPollResultNotFound {
                is_first_poll: true,
                ids: ids.clone(),
                location: loc(),
            },
            RestError::MissingResultUrl {
                ids: ids.clone(),
                location: loc(),
            },
            RestError::MissingQueryId {
                ids: ids.clone(),
                location: loc(),
            },
        ] {
            assert_eq!(
                err.snowflake_context(),
                SnowflakeErrorContext {
                    vendor_code: None,
                    sql_state: None,
                    query_id: Some("01abc".to_owned()),
                    request_id: Some(request_id.to_string()),
                },
                "ids dropped on {err:?}"
            );
        }
    }

    #[test]
    fn login_operation_timeout_gets_hyt00_without_ids() {
        let err = RestError::OperationTimeout {
            operation: "login".to_owned(),
            budget: std::time::Duration::from_secs(1),
            ids: QueryIds::default(),
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext {
                vendor_code: None,
                sql_state: Some(SQLSTATE_TIMEOUT_EXPIRED.to_string()),
                query_id: None,
                request_id: None,
            }
        );
    }

    #[test]
    fn master_token_terminal_gets_connection_sql_state() {
        let err = RestError::MasterTokenTerminal {
            code: 390114,
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext::gs_connection_not_established(390114)
        );
    }

    #[test]
    fn poll_operation_timeout_passes_through_ids() {
        let request_id = uuid::Uuid::new_v4();
        let err = RestError::OperationTimeout {
            operation: "statement poll".to_owned(),
            budget: std::time::Duration::from_secs(1),
            ids: QueryIds {
                request_id: Some(request_id),
                query_id: Some("01abc".to_owned()),
            },
            location: loc(),
        };
        assert_eq!(
            err.snowflake_context(),
            SnowflakeErrorContext {
                vendor_code: None,
                sql_state: Some(SQLSTATE_TIMEOUT_EXPIRED.to_string()),
                query_id: Some("01abc".to_owned()),
                request_id: Some(request_id.to_string()),
            }
        );
    }
}
