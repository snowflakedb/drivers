//! Exhaustive walk of [`RestError`] for vendor / SQLSTATE / query id / request id
//!
//! Adding a variant is a compile error until it is classified,
//! as there is no wildcard that silently drops those fields.

use super::error::SfError;
use super::sql_state::sql_state_from_code;
use super::{
    CREDENTIAL_REJECTION_GS_CODES, GS_CODE_UNAVAILABLE, RestError, SQLSTATE_AUTHORIZATION_FAILURE,
    SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED, SQLSTATE_TIMEOUT_EXPIRED,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SnowflakeErrorContext {
    pub vendor_code: Option<i32>,
    pub sql_state: Option<String>,
    pub query_id: Option<String>,
    pub request_id: Option<String>,
}

impl SnowflakeErrorContext {
    fn with_sql_state_fallback(mut self) -> Self {
        if self.sql_state.is_none() {
            self.sql_state = self
                .vendor_code
                .and_then(sql_state_from_code)
                .map(str::to_owned);
        }
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
    ///   3. `sql_state_from_code` lookup against the numeric Snowflake error
    ///      code, which covers paths (async-poll, query-monitoring) that drop
    ///      `sqlState` on the wire but keep the error code. Login and
    ///      master-token codes are deliberately not in that table.
    ///
    /// A code of [`GS_CODE_UNAVAILABLE`] (`-1`) means the server omitted or
    /// sent a non-numeric code; no vendor_code is surfaced. Message text is
    /// never inspected — classification belongs to the server.
    pub(crate) fn snowflake_context(&self) -> SnowflakeErrorContext {
        let ctx = match self {
            RestError::QueryFailed {
                code,
                sql_state,
                query_id,
                request_id,
                ..
            } => SnowflakeErrorContext {
                vendor_code: *code,
                sql_state: sql_state.clone(),
                query_id: query_id.clone(),
                request_id: request_id.map(|id| id.to_string()),
            },
            RestError::AsyncQuery {
                source,
                request_id,
                query_id,
                ..
            } => SnowflakeErrorContext {
                vendor_code: gs_code(source),
                sql_state: None,
                query_id: query_id.map(|id| id.to_string()),
                request_id: request_id.map(|id| id.to_string()),
            },
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
                        query_id: None,
                        request_id: None,
                    }
                }
            }
            RestError::OperationTimeout { .. } => SnowflakeErrorContext {
                vendor_code: None,
                sql_state: Some(SQLSTATE_TIMEOUT_EXPIRED.to_string()),
                query_id: None,
                request_id: None,
            },
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
            | RestError::SessionRefreshFailed { .. }
            | RestError::TokenRequestHttp { .. }
            | RestError::TokenRequestFailed { .. }
            | RestError::Heartbeat { .. }
            | RestError::MissingResponseField { .. }
            | RestError::HttpRetry { .. }
            | RestError::Logout { .. }
            | RestError::InvalidUrl { .. }
            | RestError::PayloadEncode { .. } => SnowflakeErrorContext::default(),
        };
        ctx.with_sql_state_fallback()
    }
}

fn gs_code(err: &SfError) -> Option<i32> {
    match err {
        SfError::SnowflakeBody { code, .. } => Some(*code),
        // no wildcard - explicit empty arms
        SfError::Transport { .. }
        | SfError::HttpStatus { .. }
        | SfError::AsyncPollResultNotFound { .. }
        | SfError::SessionExpired { .. }
        | SfError::WarehouseResuming { .. }
        | SfError::DeadlineExceeded { .. }
        | SfError::RetryAttemptsExhausted { .. }
        | SfError::RetryBudgetExceeded { .. }
        | SfError::MissingResultUrl { .. }
        | SfError::MissingQueryId { .. }
        | SfError::ResultUrlParse { .. }
        | SfError::Cancelled { .. }
        | SfError::BodyParse { .. } => None,
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
            query_id: Some("01abc".to_owned()),
            request_id: Some(request_id),
            location: loc(),
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
            query_id: None,
            request_id: None,
            location: loc(),
        };
        let ctx = err.snowflake_context();
        assert_eq!(ctx.vendor_code, Some(100038));
        assert_eq!(ctx.sql_state.as_deref(), Some("22003"));
    }

    #[test]
    fn login_credential_rejection_gets_authorization_sql_state() {
        let err = RestError::LoginError {
            message: "bad password".to_owned(),
            code: 390100,
            reauthentication_required: false,
            location: loc(),
        };
        let ctx = err.snowflake_context();
        assert_eq!(ctx.vendor_code, Some(390100));
        assert_eq!(ctx.sql_state.as_deref(), Some("28000"));
        assert_eq!(ctx.query_id, None);
        assert_eq!(ctx.request_id, None);
    }

    #[test]
    fn login_reauth_gets_connection_sql_state() {
        let err = RestError::LoginError {
            message: "id token expired".to_owned(),
            code: 390195,
            reauthentication_required: true,
            location: loc(),
        };
        let ctx = err.snowflake_context();
        assert_eq!(ctx.vendor_code, Some(390195));
        assert_eq!(ctx.sql_state.as_deref(), Some("08001"));
    }

    #[test]
    fn login_non_rejection_non_reauth_has_no_sql_state() {
        let err = RestError::LoginError {
            message: "session expired".to_owned(),
            code: 390111,
            reauthentication_required: false,
            location: loc(),
        };
        let ctx = err.snowflake_context();
        assert_eq!(ctx.vendor_code, Some(390111));
        assert_eq!(ctx.sql_state, None);
    }

    #[test]
    fn session_refresh_failed_does_not_surface_vendor_code() {
        // Intentionally not wired through yet — see snowflake_context docs.
        let err = RestError::SessionRefreshFailed {
            message: "expired".to_owned(),
            code: 390111,
            location: loc(),
        };
        assert_eq!(err.snowflake_context(), SnowflakeErrorContext::default());
    }

    #[test]
    fn operation_timeout_gets_hyt00_without_request_id() {
        let err = RestError::OperationTimeout {
            operation: "login".to_owned(),
            budget: std::time::Duration::from_secs(1),
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
}
