// ReSharper disable InconsistentNaming

using Snowflake.Data.Attributes;

namespace Snowflake.Data;

public enum SFError
{
    [SFError(ErrorCode = 270001)] INTERNAL_ERROR,
    [SFError(ErrorCode = 270002)] COLUMN_INDEX_OUT_OF_BOUND,
    [SFError(ErrorCode = 270003)] INVALID_DATA_CONVERSION,
    [SFError(ErrorCode = 270004)] STATEMENT_ALREADY_RUNNING_QUERY,
    [SFError(ErrorCode = 270005)] QUERY_CANCELLED,
    [SFError(ErrorCode = 270006)] MISSING_CONNECTION_PROPERTY,
    [SFError(ErrorCode = 270007)] REQUEST_TIMEOUT,
    [SFError(ErrorCode = 270008)] INVALID_CONNECTION_STRING,
    [SFError(ErrorCode = 270009)] UNSUPPORTED_FEATURE,
    [SFError(ErrorCode = 270010)] DATA_READER_ALREADY_CLOSED,
    [SFError(ErrorCode = 270011)] UNKNOWN_AUTHENTICATOR,
    [SFError(ErrorCode = 270012)] UNSUPPORTED_PLATFORM,
    [SFError(ErrorCode = 270040)] IDP_SSO_TOKEN_URL_MISMATCH,
    [SFError(ErrorCode = 270041)] IDP_SAML_POSTBACK_NOTFOUND,
    [SFError(ErrorCode = 270042)] IDP_SAML_POSTBACK_INVALID,
    [SFError(ErrorCode = 270050)] BROWSER_RESPONSE_WRONG_METHOD,
    [SFError(ErrorCode = 270051)] BROWSER_RESPONSE_INVALID_PREFIX,
    [SFError(ErrorCode = 270052)] JWT_ERROR_READING_PK,
    [SFError(ErrorCode = 270053)] UNSUPPORTED_DOTNET_TYPE,
    [SFError(ErrorCode = 270054)] UNSUPPORTED_SNOWFLAKE_TYPE_FOR_PARAM,
    [SFError(ErrorCode = 270055)] INVALID_CONNECTION_PARAMETER_VALUE,
    [SFError(ErrorCode = 270056)] INVALID_BROWSER_URL,
    [SFError(ErrorCode = 270057)] BROWSER_RESPONSE_TIMEOUT,
    [SFError(ErrorCode = 270058)] IO_ERROR_ON_GETPUT_COMMAND,
    [SFError(ErrorCode = 270059)] EXECUTE_COMMAND_ON_CLOSED_CONNECTION,
    [SFError(ErrorCode = 270060)] INCONSISTENT_RESULT_ERROR,
    [SFError(ErrorCode = 270061)] STRUCTURED_TYPE_READ_ERROR,
    [SFError(ErrorCode = 270062)] STRUCTURED_TYPE_READ_DETAILED_ERROR,
    [SFError(ErrorCode = 270063)] BROWSER_RESPONSE_ERROR,
    [SFError(ErrorCode = 270064)] OAUTH_TOKEN_REQUEST_ERROR,
    [SFError(ErrorCode = 270065)] EXPERIMENTAL_AUTHENTICATION_DISABLED,
    [SFError(ErrorCode = 270066)] WIF_ATTESTATION_ERROR,
    [SFError(ErrorCode = 270067)] TLS_CONFIGURATION_NOT_SUPPORTED,
    [SFError(ErrorCode = 270068)] CREDENTIAL_WRITE_ERROR,
    // TODO(SNOW-4382591): add messages for the codes below. The classic driver threw a bare System.Exception with:
    //   COMMAND_TEXT_NOT_SET => "Unable to execute command due to command text not being set"
    //   INVALID_QUERY_ID_FORMAT => "Invalid query id format. Expected a UUID."
    //   UNKNOWN_QUERY_STATUS => "The query status returned by the server is not recognized"
    //   QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED => "Max retry for no data is reached"
    [SFError(ErrorCode = 270069)] COMMAND_TEXT_NOT_SET,
    [SFError(ErrorCode = 270070)] INVALID_QUERY_ID_FORMAT,
    [SFError(ErrorCode = 270071)] UNKNOWN_QUERY_STATUS,
    [SFError(ErrorCode = 270072)] QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED,
    [SFError(ErrorCode = 390111)] SESSION_GONE,
    [SFError(ErrorCode = 390120)] EXT_AUTHN_DENIED,
    [SFError(ErrorCode = 390123)] EXT_AUTHN_LOCKED,
    [SFError(ErrorCode = 390126)] EXT_AUTHN_TIMEOUT,
    [SFError(ErrorCode = 390127)] EXT_AUTHN_INVALID,
    [SFError(ErrorCode = 390129)] EXT_AUTHN_EXCEPTION,
    [SFError(ErrorCode = 390201)] OBJ_NONEXISTANT_OR_NOT_AUTHORIZED,
    [SFError(ErrorCode = 390318)] EXT_OAUTH_ACCESS_TOKEN_EXPIRED,
    [SFError(ErrorCode = 390303)] EXT_OAUTH_ACCESS_TOKEN_INVALID,
    [SFError(ErrorCode = 390195)] ID_TOKEN_INVALID,
}
