//! ODBC SQLState codes as defined by the ODBC specification
//!
//! This module provides a comprehensive enum for all standard ODBC SQLState codes
//! with conversions to and from string representations.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

/// ODBC SQLState codes as defined by the ODBC specification
///
/// SQLState is a five-character string that indicates the success or failure
/// of an SQL operation. The first two characters indicate the class,
/// and the last three characters indicate the subclass.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum SqlState {
    // Success class (00)
    /// 00000 - Successful completion
    #[default]
    SuccessfulCompletion,

    // Warning class (01)
    /// 01000 - General warning
    GeneralWarning,
    /// 01001 - Cursor operation conflict
    CursorOperationConflict,
    /// 01002 - Disconnect error
    DisconnectError,
    /// 01003 - NULL value eliminated in set function
    NullValueEliminatedInSetFunction,
    /// 01004 - String data, right truncated
    StringDataRightTruncated,
    /// 01006 - Privilege not revoked
    PrivilegeNotRevoked,
    /// 01007 - Privilege not granted
    PrivilegeNotGranted,
    /// 01S00 - Invalid connection string attribute
    InvalidConnectionStringAttribute,
    /// 01S01 - Error in row
    ErrorInRow,
    /// 01S02 - Option value changed
    OptionValueChanged,
    /// 01S03 - No rows updated or deleted
    NoRowsUpdatedOrDeleted,
    /// 01S04 - More than one row updated or deleted
    MoreThanOneRowUpdatedOrDeleted,
    /// 01S05 - Cancel treated as FreeStmt/Close
    CancelTreatedAsFreeStmtClose,
    /// 01S06 - Attempt to fetch before the result set returned the first rowset
    AttemptToFetchBeforeFirstRowset,
    /// 01S07 - Fractional truncation
    FractionalTruncation,
    /// 01S08 - Error saving File DSN
    ErrorSavingFileDsn,
    /// 01S09 - Invalid keyword
    InvalidKeyword,

    // No data class (02)
    /// 02000 - No data found
    NoDataFound,

    // Dynamic SQL error class (07)
    /// 07001 - Wrong number of parameters
    WrongNumberOfParameters,
    /// 07002 - COUNT field incorrect
    CountFieldIncorrect,
    /// 07005 - Prepared statement not a cursor-specification
    PreparedStatementNotCursorSpecification,
    /// 07006 - Restricted data type attribute violation
    RestrictedDataTypeAttributeViolation,
    /// 07009 - Invalid descriptor index
    InvalidDescriptorIndex,
    /// 07S01 - Invalid use of default parameter
    InvalidUseOfDefaultParameter,

    // Connection exception class (08)
    /// 08001 - Client unable to establish connection
    ClientUnableToEstablishConnection,
    /// 08002 - Connection name in use
    ConnectionNameInUse,
    /// 08003 - Connection does not exist
    ConnectionDoesNotExist,
    /// 08004 - Server rejected the connection
    ServerRejectedConnection,
    /// 08007 - Connection failure during transaction
    ConnectionFailureDuringTransaction,
    /// 08S01 - Communication link failure
    CommunicationLinkFailure,

    // Triggered action exception class (09)
    /// 09000 - Triggered action exception
    TriggeredActionException,

    // Feature not supported class (0A)
    /// 0A000 - Feature not supported
    FeatureNotSupported,

    // Invalid transaction state class (25)
    /// 25000 - Invalid transaction state
    InvalidTransactionState,
    /// 25S01 - Transaction state unknown
    TransactionStateUnknown,
    /// 25S02 - Transaction is still active
    TransactionIsStillActive,
    /// 25S03 - Transaction is rolled back
    TransactionIsRolledBack,

    // Invalid SQL statement name class (26)
    /// 26000 - Invalid SQL statement name
    InvalidSqlStatementName,

    // Triggered data change violation class (27)
    /// 27000 - Triggered data change violation
    TriggeredDataChangeViolation,

    // Invalid authorization specification class (28)
    /// 28000 - Invalid authorization specification
    InvalidAuthorizationSpecification,

    // Direct SQL syntax error or access rule violation class (2A)
    /// 2A000 - Direct SQL syntax error or access rule violation
    DirectSqlSyntaxErrorOrAccessRuleViolation,

    // Dependent privilege descriptors still exist class (2B)
    /// 2B000 - Dependent privilege descriptors still exist
    DependentPrivilegeDescriptorsStillExist,

    // Invalid character set name class (2C)
    /// 2C000 - Invalid character set name
    InvalidCharacterSetName,

    // Invalid transaction termination class (2D)
    /// 2D000 - Invalid transaction termination
    InvalidTransactionTermination,

    // Invalid connection name class (2E)
    /// 2E000 - Invalid connection name
    InvalidConnectionName,

    // SQL routine exception class (2F)
    /// 2F000 - SQL routine exception
    SqlRoutineException,
    /// 2F002 - Modifying SQL data not permitted
    ModifyingSqlDataNotPermitted,
    /// 2F003 - Prohibited SQL statement attempted
    ProhibitedSqlStatementAttempted,
    /// 2F004 - Reading SQL data not permitted
    ReadingSqlDataNotPermitted,
    /// 2F005 - Function executed no return statement
    FunctionExecutedNoReturnStatement,

    // Invalid collation name class (2H)
    /// 2H000 - Invalid collation name
    InvalidCollationName,

    // Syntax error or access rule violation class (42)
    /// 42000 - Syntax error or access rule violation
    SyntaxErrorOrAccessRuleViolation,
    /// 42S01 - Base table or view already exists
    BaseTableOrViewAlreadyExists,
    /// 42S02 - Base table or view not found
    BaseTableOrViewNotFound,
    /// 42S11 - Index already exists
    IndexAlreadyExists,
    /// 42S12 - Index not found
    IndexNotFound,
    /// 42S21 - Column already exists
    ColumnAlreadyExists,
    /// 42S22 - Column not found
    ColumnNotFound,

    // Check option violation class (44)
    /// 44000 - WITH CHECK OPTION violation
    WithCheckOptionViolation,

    // CLI-specific condition class (HY)
    /// HY000 - General error
    GeneralError,
    /// HY001 - Memory allocation error
    MemoryAllocationError,
    /// HY003 - Invalid application buffer type
    InvalidApplicationBufferType,
    /// HY004 - Invalid SQL data type
    InvalidSqlDataType,
    /// HY007 - Associated statement is not prepared
    AssociatedStatementIsNotPrepared,
    /// HY008 - Operation canceled
    OperationCanceled,
    /// HY009 - Invalid use of null pointer
    InvalidUseOfNullPointer,
    /// HY010 - Function sequence error
    FunctionSequenceError,
    /// HY011 - Attribute cannot be set now
    AttributeCannotBeSetNow,
    /// HY012 - Invalid transaction operation code
    InvalidTransactionOperationCode,
    /// HY013 - Memory management error
    MemoryManagementError,
    /// HY014 - Limit on the number of handles exceeded
    LimitOnNumberOfHandlesExceeded,
    /// HY015 - No cursor name available
    NoCursorNameAvailable,
    /// HY016 - Cannot modify an implementation row descriptor
    CannotModifyImplementationRowDescriptor,
    /// HY017 - Invalid use of an automatically allocated descriptor handle
    InvalidUseOfAutomaticallyAllocatedDescriptorHandle,
    /// HY018 - Server declined cancel request
    ServerDeclinedCancelRequest,
    /// HY019 - Non-character and non-binary data sent in pieces
    NonCharacterAndNonBinaryDataSentInPieces,
    /// HY020 - Attempt to concatenate a null value
    AttemptToConcatenateNullValue,
    /// HY021 - Inconsistent descriptor information
    InconsistentDescriptorInformation,
    /// HY024 - Invalid attribute value
    InvalidAttributeValue,
    /// HY090 - Invalid string or buffer length
    InvalidStringOrBufferLength,
    /// HY091 - Invalid descriptor field identifier
    InvalidDescriptorFieldIdentifier,
    /// HY092 - Invalid attribute/option identifier
    InvalidAttributeOptionIdentifier,
    /// HY095 - Function type out of range
    FunctionTypeOutOfRange,
    /// HY096 - Invalid information type
    InvalidInformationType,
    /// HY097 - Column type out of range
    ColumnTypeOutOfRange,
    /// HY098 - Scope type out of range
    ScopeTypeOutOfRange,
    /// HY099 - Nullable type out of range
    NullableTypeOutOfRange,
    /// HY100 - Uniqueness option type out of range
    UniquenessOptionTypeOutOfRange,
    /// HY101 - Accuracy option type out of range
    AccuracyOptionTypeOutOfRange,
    /// HY103 - Invalid retrieval code
    InvalidRetrievalCode,
    /// HY104 - Invalid precision or scale value
    InvalidPrecisionOrScaleValue,
    /// HY105 - Invalid parameter type
    InvalidParameterType,
    /// HY106 - Fetch type out of range
    FetchTypeOutOfRange,
    /// HY107 - Row value out of range
    RowValueOutOfRange,
    /// HY108 - Concurrency option out of range
    ConcurrencyOptionOutOfRange,
    /// HY109 - Invalid cursor position
    InvalidCursorPosition,
    /// HY110 - Invalid driver completion
    InvalidDriverCompletion,
    /// HY111 - Invalid bookmark value
    InvalidBookmarkValue,
    /// HYC00 - Optional feature not implemented
    OptionalFeatureNotImplemented,
    /// HYT00 - Timeout expired
    TimeoutExpired,
    /// HYT01 - Connection timeout expired
    ConnectionTimeoutExpired,

    // Driver-specific class (IM)
    /// IM001 - Driver does not support this function
    DriverDoesNotSupportThisFunction,
    /// IM002 - Data source name not found and no default driver specified
    DataSourceNameNotFoundAndNoDefaultDriverSpecified,
    /// IM003 - Specified driver could not be loaded
    SpecifiedDriverCouldNotBeLoaded,
    /// IM004 - Driver's SQLAllocHandle on SQL_HANDLE_ENV failed
    DriverSqlAllocHandleOnSqlHandleEnvFailed,
    /// IM005 - Driver's SQLAllocHandle on SQL_HANDLE_DBC failed
    DriverSqlAllocHandleOnSqlHandleDbcFailed,
    /// IM006 - Driver's SQLSetConnectAttr failed
    DriverSqlSetConnectAttrFailed,
    /// IM007 - No data source or driver specified; dialog prohibited
    NoDataSourceOrDriverSpecifiedDialogProhibited,
    /// IM008 - Dialog failed
    DialogFailed,
    /// IM009 - Unable to load translation DLL
    UnableToLoadTranslationDll,
    /// IM010 - Data source name too long
    DataSourceNameTooLong,
    /// IM011 - Driver name too long
    DriverNameTooLong,
    /// IM012 - DRIVER keyword syntax error
    DriverKeywordSyntaxError,
    /// IM013 - Trace file error
    TraceFileError,
    /// IM014 - Invalid name of File DSN
    InvalidNameOfFileDsn,
    /// IM015 - Corrupt file data source
    CorruptFileDataSource,

    // Unknown or custom SQLState
    /// Unknown SQLState code
    Unknown(String),
}

impl SqlState {
    /// Returns the SQL state code as a string
    pub fn as_str(&self) -> &str {
        match self {
            SqlState::SuccessfulCompletion => "00000",
            SqlState::GeneralWarning => "01000",
            SqlState::CursorOperationConflict => "01001",
            SqlState::DisconnectError => "01002",
            SqlState::NullValueEliminatedInSetFunction => "01003",
            SqlState::StringDataRightTruncated => "01004",
            SqlState::PrivilegeNotRevoked => "01006",
            SqlState::PrivilegeNotGranted => "01007",
            SqlState::InvalidConnectionStringAttribute => "01S00",
            SqlState::ErrorInRow => "01S01",
            SqlState::OptionValueChanged => "01S02",
            SqlState::NoRowsUpdatedOrDeleted => "01S03",
            SqlState::MoreThanOneRowUpdatedOrDeleted => "01S04",
            SqlState::CancelTreatedAsFreeStmtClose => "01S05",
            SqlState::AttemptToFetchBeforeFirstRowset => "01S06",
            SqlState::FractionalTruncation => "01S07",
            SqlState::ErrorSavingFileDsn => "01S08",
            SqlState::InvalidKeyword => "01S09",
            SqlState::NoDataFound => "02000",
            SqlState::WrongNumberOfParameters => "07001",
            SqlState::CountFieldIncorrect => "07002",
            SqlState::PreparedStatementNotCursorSpecification => "07005",
            SqlState::RestrictedDataTypeAttributeViolation => "07006",
            SqlState::InvalidDescriptorIndex => "07009",
            SqlState::InvalidUseOfDefaultParameter => "07S01",
            SqlState::ClientUnableToEstablishConnection => "08001",
            SqlState::ConnectionNameInUse => "08002",
            SqlState::ConnectionDoesNotExist => "08003",
            SqlState::ServerRejectedConnection => "08004",
            SqlState::ConnectionFailureDuringTransaction => "08007",
            SqlState::CommunicationLinkFailure => "08S01",
            SqlState::TriggeredActionException => "09000",
            SqlState::FeatureNotSupported => "0A000",
            SqlState::InvalidTransactionState => "25000",
            SqlState::TransactionStateUnknown => "25S01",
            SqlState::TransactionIsStillActive => "25S02",
            SqlState::TransactionIsRolledBack => "25S03",
            SqlState::InvalidSqlStatementName => "26000",
            SqlState::TriggeredDataChangeViolation => "27000",
            SqlState::InvalidAuthorizationSpecification => "28000",
            SqlState::DirectSqlSyntaxErrorOrAccessRuleViolation => "2A000",
            SqlState::DependentPrivilegeDescriptorsStillExist => "2B000",
            SqlState::InvalidCharacterSetName => "2C000",
            SqlState::InvalidTransactionTermination => "2D000",
            SqlState::InvalidConnectionName => "2E000",
            SqlState::SqlRoutineException => "2F000",
            SqlState::ModifyingSqlDataNotPermitted => "2F002",
            SqlState::ProhibitedSqlStatementAttempted => "2F003",
            SqlState::ReadingSqlDataNotPermitted => "2F004",
            SqlState::FunctionExecutedNoReturnStatement => "2F005",
            SqlState::InvalidCollationName => "2H000",
            SqlState::SyntaxErrorOrAccessRuleViolation => "42000",
            SqlState::BaseTableOrViewAlreadyExists => "42S01",
            SqlState::BaseTableOrViewNotFound => "42S02",
            SqlState::IndexAlreadyExists => "42S11",
            SqlState::IndexNotFound => "42S12",
            SqlState::ColumnAlreadyExists => "42S21",
            SqlState::ColumnNotFound => "42S22",
            SqlState::WithCheckOptionViolation => "44000",
            SqlState::GeneralError => "HY000",
            SqlState::MemoryAllocationError => "HY001",
            SqlState::InvalidApplicationBufferType => "HY003",
            SqlState::InvalidSqlDataType => "HY004",
            SqlState::AssociatedStatementIsNotPrepared => "HY007",
            SqlState::OperationCanceled => "HY008",
            SqlState::InvalidUseOfNullPointer => "HY009",
            SqlState::FunctionSequenceError => "HY010",
            SqlState::AttributeCannotBeSetNow => "HY011",
            SqlState::InvalidTransactionOperationCode => "HY012",
            SqlState::MemoryManagementError => "HY013",
            SqlState::LimitOnNumberOfHandlesExceeded => "HY014",
            SqlState::NoCursorNameAvailable => "HY015",
            SqlState::CannotModifyImplementationRowDescriptor => "HY016",
            SqlState::InvalidUseOfAutomaticallyAllocatedDescriptorHandle => "HY017",
            SqlState::ServerDeclinedCancelRequest => "HY018",
            SqlState::NonCharacterAndNonBinaryDataSentInPieces => "HY019",
            SqlState::AttemptToConcatenateNullValue => "HY020",
            SqlState::InconsistentDescriptorInformation => "HY021",
            SqlState::InvalidAttributeValue => "HY024",
            SqlState::InvalidStringOrBufferLength => "HY090",
            SqlState::InvalidDescriptorFieldIdentifier => "HY091",
            SqlState::InvalidAttributeOptionIdentifier => "HY092",
            SqlState::FunctionTypeOutOfRange => "HY095",
            SqlState::InvalidInformationType => "HY096",
            SqlState::ColumnTypeOutOfRange => "HY097",
            SqlState::ScopeTypeOutOfRange => "HY098",
            SqlState::NullableTypeOutOfRange => "HY099",
            SqlState::UniquenessOptionTypeOutOfRange => "HY100",
            SqlState::AccuracyOptionTypeOutOfRange => "HY101",
            SqlState::InvalidRetrievalCode => "HY103",
            SqlState::InvalidPrecisionOrScaleValue => "HY104",
            SqlState::InvalidParameterType => "HY105",
            SqlState::FetchTypeOutOfRange => "HY106",
            SqlState::RowValueOutOfRange => "HY107",
            SqlState::ConcurrencyOptionOutOfRange => "HY108",
            SqlState::InvalidCursorPosition => "HY109",
            SqlState::InvalidDriverCompletion => "HY110",
            SqlState::InvalidBookmarkValue => "HY111",
            SqlState::OptionalFeatureNotImplemented => "HYC00",
            SqlState::TimeoutExpired => "HYT00",
            SqlState::ConnectionTimeoutExpired => "HYT01",
            SqlState::DriverDoesNotSupportThisFunction => "IM001",
            SqlState::DataSourceNameNotFoundAndNoDefaultDriverSpecified => "IM002",
            SqlState::SpecifiedDriverCouldNotBeLoaded => "IM003",
            SqlState::DriverSqlAllocHandleOnSqlHandleEnvFailed => "IM004",
            SqlState::DriverSqlAllocHandleOnSqlHandleDbcFailed => "IM005",
            SqlState::DriverSqlSetConnectAttrFailed => "IM006",
            SqlState::NoDataSourceOrDriverSpecifiedDialogProhibited => "IM007",
            SqlState::DialogFailed => "IM008",
            SqlState::UnableToLoadTranslationDll => "IM009",
            SqlState::DataSourceNameTooLong => "IM010",
            SqlState::DriverNameTooLong => "IM011",
            SqlState::DriverKeywordSyntaxError => "IM012",
            SqlState::TraceFileError => "IM013",
            SqlState::InvalidNameOfFileDsn => "IM014",
            SqlState::CorruptFileDataSource => "IM015",
            SqlState::Unknown(code) => code,
        }
    }

    /// Returns true if this SQLState represents a successful operation
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        matches!(self, SqlState::SuccessfulCompletion)
    }

    /// Returns true if this SQLState represents a warning
    #[allow(dead_code)]
    pub fn is_warning(&self) -> bool {
        matches!(
            self,
            SqlState::GeneralWarning
                | SqlState::CursorOperationConflict
                | SqlState::DisconnectError
                | SqlState::NullValueEliminatedInSetFunction
                | SqlState::StringDataRightTruncated
                | SqlState::PrivilegeNotRevoked
                | SqlState::PrivilegeNotGranted
                | SqlState::InvalidConnectionStringAttribute
                | SqlState::ErrorInRow
                | SqlState::OptionValueChanged
                | SqlState::NoRowsUpdatedOrDeleted
                | SqlState::MoreThanOneRowUpdatedOrDeleted
                | SqlState::CancelTreatedAsFreeStmtClose
                | SqlState::AttemptToFetchBeforeFirstRowset
                | SqlState::FractionalTruncation
                | SqlState::ErrorSavingFileDsn
                | SqlState::InvalidKeyword
        )
    }

    /// Returns true if this SQLState represents an error
    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        !self.is_success() && !self.is_warning()
    }
}

impl Display for SqlState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SqlState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sql_state = match s {
            "00000" => SqlState::SuccessfulCompletion,
            "01000" => SqlState::GeneralWarning,
            "01001" => SqlState::CursorOperationConflict,
            "01002" => SqlState::DisconnectError,
            "01003" => SqlState::NullValueEliminatedInSetFunction,
            "01004" => SqlState::StringDataRightTruncated,
            "01006" => SqlState::PrivilegeNotRevoked,
            "01007" => SqlState::PrivilegeNotGranted,
            "01S00" => SqlState::InvalidConnectionStringAttribute,
            "01S01" => SqlState::ErrorInRow,
            "01S02" => SqlState::OptionValueChanged,
            "01S03" => SqlState::NoRowsUpdatedOrDeleted,
            "01S04" => SqlState::MoreThanOneRowUpdatedOrDeleted,
            "01S05" => SqlState::CancelTreatedAsFreeStmtClose,
            "01S06" => SqlState::AttemptToFetchBeforeFirstRowset,
            "01S07" => SqlState::FractionalTruncation,
            "01S08" => SqlState::ErrorSavingFileDsn,
            "01S09" => SqlState::InvalidKeyword,
            "02000" => SqlState::NoDataFound,
            "07001" => SqlState::WrongNumberOfParameters,
            "07002" => SqlState::CountFieldIncorrect,
            "07005" => SqlState::PreparedStatementNotCursorSpecification,
            "07006" => SqlState::RestrictedDataTypeAttributeViolation,
            "07009" => SqlState::InvalidDescriptorIndex,
            "07S01" => SqlState::InvalidUseOfDefaultParameter,
            "08001" => SqlState::ClientUnableToEstablishConnection,
            "08002" => SqlState::ConnectionNameInUse,
            "08003" => SqlState::ConnectionDoesNotExist,
            "08004" => SqlState::ServerRejectedConnection,
            "08007" => SqlState::ConnectionFailureDuringTransaction,
            "08S01" => SqlState::CommunicationLinkFailure,
            "09000" => SqlState::TriggeredActionException,
            "0A000" => SqlState::FeatureNotSupported,
            "25000" => SqlState::InvalidTransactionState,
            "25S01" => SqlState::TransactionStateUnknown,
            "25S02" => SqlState::TransactionIsStillActive,
            "25S03" => SqlState::TransactionIsRolledBack,
            "26000" => SqlState::InvalidSqlStatementName,
            "27000" => SqlState::TriggeredDataChangeViolation,
            "28000" => SqlState::InvalidAuthorizationSpecification,
            "2A000" => SqlState::DirectSqlSyntaxErrorOrAccessRuleViolation,
            "2B000" => SqlState::DependentPrivilegeDescriptorsStillExist,
            "2C000" => SqlState::InvalidCharacterSetName,
            "2D000" => SqlState::InvalidTransactionTermination,
            "2E000" => SqlState::InvalidConnectionName,
            "2F000" => SqlState::SqlRoutineException,
            "2F002" => SqlState::ModifyingSqlDataNotPermitted,
            "2F003" => SqlState::ProhibitedSqlStatementAttempted,
            "2F004" => SqlState::ReadingSqlDataNotPermitted,
            "2F005" => SqlState::FunctionExecutedNoReturnStatement,
            "2H000" => SqlState::InvalidCollationName,
            "42000" => SqlState::SyntaxErrorOrAccessRuleViolation,
            "42S01" => SqlState::BaseTableOrViewAlreadyExists,
            "42S02" => SqlState::BaseTableOrViewNotFound,
            "42S11" => SqlState::IndexAlreadyExists,
            "42S12" => SqlState::IndexNotFound,
            "42S21" => SqlState::ColumnAlreadyExists,
            "42S22" => SqlState::ColumnNotFound,
            "44000" => SqlState::WithCheckOptionViolation,
            "HY000" => SqlState::GeneralError,
            "HY001" => SqlState::MemoryAllocationError,
            "HY003" => SqlState::InvalidApplicationBufferType,
            "HY004" => SqlState::InvalidSqlDataType,
            "HY007" => SqlState::AssociatedStatementIsNotPrepared,
            "HY008" => SqlState::OperationCanceled,
            "HY009" => SqlState::InvalidUseOfNullPointer,
            "HY010" => SqlState::FunctionSequenceError,
            "HY011" => SqlState::AttributeCannotBeSetNow,
            "HY012" => SqlState::InvalidTransactionOperationCode,
            "HY013" => SqlState::MemoryManagementError,
            "HY014" => SqlState::LimitOnNumberOfHandlesExceeded,
            "HY015" => SqlState::NoCursorNameAvailable,
            "HY016" => SqlState::CannotModifyImplementationRowDescriptor,
            "HY017" => SqlState::InvalidUseOfAutomaticallyAllocatedDescriptorHandle,
            "HY018" => SqlState::ServerDeclinedCancelRequest,
            "HY019" => SqlState::NonCharacterAndNonBinaryDataSentInPieces,
            "HY020" => SqlState::AttemptToConcatenateNullValue,
            "HY021" => SqlState::InconsistentDescriptorInformation,
            "HY024" => SqlState::InvalidAttributeValue,
            "HY090" => SqlState::InvalidStringOrBufferLength,
            "HY091" => SqlState::InvalidDescriptorFieldIdentifier,
            "HY092" => SqlState::InvalidAttributeOptionIdentifier,
            "HY095" => SqlState::FunctionTypeOutOfRange,
            "HY096" => SqlState::InvalidInformationType,
            "HY097" => SqlState::ColumnTypeOutOfRange,
            "HY098" => SqlState::ScopeTypeOutOfRange,
            "HY099" => SqlState::NullableTypeOutOfRange,
            "HY100" => SqlState::UniquenessOptionTypeOutOfRange,
            "HY101" => SqlState::AccuracyOptionTypeOutOfRange,
            "HY103" => SqlState::InvalidRetrievalCode,
            "HY104" => SqlState::InvalidPrecisionOrScaleValue,
            "HY105" => SqlState::InvalidParameterType,
            "HY106" => SqlState::FetchTypeOutOfRange,
            "HY107" => SqlState::RowValueOutOfRange,
            "HY108" => SqlState::ConcurrencyOptionOutOfRange,
            "HY109" => SqlState::InvalidCursorPosition,
            "HY110" => SqlState::InvalidDriverCompletion,
            "HY111" => SqlState::InvalidBookmarkValue,
            "HYC00" => SqlState::OptionalFeatureNotImplemented,
            "HYT00" => SqlState::TimeoutExpired,
            "HYT01" => SqlState::ConnectionTimeoutExpired,
            "IM001" => SqlState::DriverDoesNotSupportThisFunction,
            "IM002" => SqlState::DataSourceNameNotFoundAndNoDefaultDriverSpecified,
            "IM003" => SqlState::SpecifiedDriverCouldNotBeLoaded,
            "IM004" => SqlState::DriverSqlAllocHandleOnSqlHandleEnvFailed,
            "IM005" => SqlState::DriverSqlAllocHandleOnSqlHandleDbcFailed,
            "IM006" => SqlState::DriverSqlSetConnectAttrFailed,
            "IM007" => SqlState::NoDataSourceOrDriverSpecifiedDialogProhibited,
            "IM008" => SqlState::DialogFailed,
            "IM009" => SqlState::UnableToLoadTranslationDll,
            "IM010" => SqlState::DataSourceNameTooLong,
            "IM011" => SqlState::DriverNameTooLong,
            "IM012" => SqlState::DriverKeywordSyntaxError,
            "IM013" => SqlState::TraceFileError,
            "IM014" => SqlState::InvalidNameOfFileDsn,
            "IM015" => SqlState::CorruptFileDataSource,
            _ => SqlState::Unknown(s.to_string()),
        };
        Ok(sql_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_state_to_string() {
        assert_eq!(SqlState::SuccessfulCompletion.to_string(), "00000");
        assert_eq!(SqlState::GeneralWarning.to_string(), "01000");
        assert_eq!(SqlState::GeneralError.to_string(), "HY000");
        assert_eq!(
            SqlState::DriverDoesNotSupportThisFunction.to_string(),
            "IM001"
        );
    }

    #[test]
    fn test_as_str() {
        assert_eq!(SqlState::SuccessfulCompletion.as_str(), "00000");
        assert_eq!(SqlState::StringDataRightTruncated.as_str(), "01004");
        assert_eq!(SqlState::NoDataFound.as_str(), "02000");
        assert_eq!(SqlState::WrongNumberOfParameters.as_str(), "07001");
        assert_eq!(
            SqlState::ClientUnableToEstablishConnection.as_str(),
            "08001"
        );
        assert_eq!(SqlState::FeatureNotSupported.as_str(), "0A000");
        assert_eq!(SqlState::InvalidTransactionState.as_str(), "25000");
        assert_eq!(SqlState::SyntaxErrorOrAccessRuleViolation.as_str(), "42000");
        assert_eq!(SqlState::GeneralError.as_str(), "HY000");
        assert_eq!(SqlState::MemoryAllocationError.as_str(), "HY001");
        assert_eq!(SqlState::OptionalFeatureNotImplemented.as_str(), "HYC00");
        assert_eq!(SqlState::TimeoutExpired.as_str(), "HYT00");
        assert_eq!(SqlState::DriverDoesNotSupportThisFunction.as_str(), "IM001");
    }

    #[test]
    fn test_string_to_sql_state() {
        assert_eq!(
            "00000".parse::<SqlState>().unwrap(),
            SqlState::SuccessfulCompletion
        );
        assert_eq!(
            "01000".parse::<SqlState>().unwrap(),
            SqlState::GeneralWarning
        );
        assert_eq!(
            "01004".parse::<SqlState>().unwrap(),
            SqlState::StringDataRightTruncated
        );
        assert_eq!("02000".parse::<SqlState>().unwrap(), SqlState::NoDataFound);
        assert_eq!(
            "07001".parse::<SqlState>().unwrap(),
            SqlState::WrongNumberOfParameters
        );
        assert_eq!(
            "08001".parse::<SqlState>().unwrap(),
            SqlState::ClientUnableToEstablishConnection
        );
        assert_eq!(
            "0A000".parse::<SqlState>().unwrap(),
            SqlState::FeatureNotSupported
        );
        assert_eq!(
            "25000".parse::<SqlState>().unwrap(),
            SqlState::InvalidTransactionState
        );
        assert_eq!(
            "42000".parse::<SqlState>().unwrap(),
            SqlState::SyntaxErrorOrAccessRuleViolation
        );
        assert_eq!("HY000".parse::<SqlState>().unwrap(), SqlState::GeneralError);
        assert_eq!(
            "HY001".parse::<SqlState>().unwrap(),
            SqlState::MemoryAllocationError
        );
        assert_eq!(
            "HYC00".parse::<SqlState>().unwrap(),
            SqlState::OptionalFeatureNotImplemented
        );
        assert_eq!(
            "HYT00".parse::<SqlState>().unwrap(),
            SqlState::TimeoutExpired
        );
        assert_eq!(
            "IM001".parse::<SqlState>().unwrap(),
            SqlState::DriverDoesNotSupportThisFunction
        );
    }

    #[test]
    fn test_unknown_sql_state() {
        let unknown_code = "XXXXX";
        let sql_state = unknown_code.parse::<SqlState>().unwrap();
        assert!(matches!(sql_state, SqlState::Unknown(_)));
        assert_eq!(sql_state.to_string(), unknown_code);
        assert_eq!(sql_state.as_str(), unknown_code);

        let custom_code = "ZZABC";
        let custom_sql_state = SqlState::Unknown(custom_code.to_string());
        assert_eq!(custom_sql_state.as_str(), custom_code);
    }

    #[test]
    fn test_is_success() {
        assert!(SqlState::SuccessfulCompletion.is_success());
        assert!(!SqlState::GeneralWarning.is_success());
        assert!(!SqlState::GeneralError.is_success());
        assert!(!SqlState::NoDataFound.is_success());
        assert!(!SqlState::StringDataRightTruncated.is_success());
    }

    #[test]
    fn test_is_warning() {
        assert!(!SqlState::SuccessfulCompletion.is_warning());
        assert!(SqlState::GeneralWarning.is_warning());
        assert!(SqlState::StringDataRightTruncated.is_warning());
        assert!(SqlState::DisconnectError.is_warning());
        assert!(SqlState::PrivilegeNotRevoked.is_warning());
        assert!(SqlState::ErrorInRow.is_warning());
        assert!(SqlState::OptionValueChanged.is_warning());
        assert!(!SqlState::GeneralError.is_warning());
        assert!(!SqlState::NoDataFound.is_warning());
    }

    #[test]
    fn test_is_error() {
        assert!(!SqlState::SuccessfulCompletion.is_error());
        assert!(!SqlState::GeneralWarning.is_error());
        assert!(!SqlState::StringDataRightTruncated.is_error());
        assert!(SqlState::GeneralError.is_error());
        assert!(SqlState::NoDataFound.is_error());
        assert!(SqlState::WrongNumberOfParameters.is_error());
        assert!(SqlState::ClientUnableToEstablishConnection.is_error());
        assert!(SqlState::FeatureNotSupported.is_error());
        assert!(SqlState::SyntaxErrorOrAccessRuleViolation.is_error());
        assert!(SqlState::MemoryAllocationError.is_error());
        assert!(SqlState::DriverDoesNotSupportThisFunction.is_error());
    }

    #[test]
    fn test_roundtrip_conversion() {
        let test_cases = vec![
            SqlState::SuccessfulCompletion,
            SqlState::GeneralWarning,
            SqlState::StringDataRightTruncated,
            SqlState::NoDataFound,
            SqlState::WrongNumberOfParameters,
            SqlState::ClientUnableToEstablishConnection,
            SqlState::FeatureNotSupported,
            SqlState::InvalidTransactionState,
            SqlState::SyntaxErrorOrAccessRuleViolation,
            SqlState::GeneralError,
            SqlState::MemoryAllocationError,
            SqlState::OptionalFeatureNotImplemented,
            SqlState::TimeoutExpired,
            SqlState::DriverDoesNotSupportThisFunction,
        ];

        for sql_state in test_cases {
            let string_repr = sql_state.to_string();
            let parsed_back = string_repr.parse::<SqlState>().unwrap();
            assert_eq!(sql_state, parsed_back);
        }
    }

    #[test]
    fn test_warning_classification() {
        // Test all 01xxx warning codes
        let warning_states = vec![
            SqlState::GeneralWarning,
            SqlState::CursorOperationConflict,
            SqlState::DisconnectError,
            SqlState::NullValueEliminatedInSetFunction,
            SqlState::StringDataRightTruncated,
            SqlState::PrivilegeNotRevoked,
            SqlState::PrivilegeNotGranted,
            SqlState::InvalidConnectionStringAttribute,
            SqlState::ErrorInRow,
            SqlState::OptionValueChanged,
            SqlState::NoRowsUpdatedOrDeleted,
            SqlState::MoreThanOneRowUpdatedOrDeleted,
            SqlState::CancelTreatedAsFreeStmtClose,
            SqlState::AttemptToFetchBeforeFirstRowset,
            SqlState::FractionalTruncation,
            SqlState::ErrorSavingFileDsn,
            SqlState::InvalidKeyword,
        ];

        for state in warning_states {
            assert!(state.is_warning(), "Expected {state:?} to be a warning");
            assert!(!state.is_success(), "Expected {state:?} to not be success");
            assert!(!state.is_error(), "Expected {state:?} to not be an error");
        }
    }

    #[test]
    fn test_common_error_states() {
        let error_states = vec![
            (SqlState::NoDataFound, "02000"),
            (SqlState::WrongNumberOfParameters, "07001"),
            (SqlState::ClientUnableToEstablishConnection, "08001"),
            (SqlState::FeatureNotSupported, "0A000"),
            (SqlState::InvalidTransactionState, "25000"),
            (SqlState::SyntaxErrorOrAccessRuleViolation, "42000"),
            (SqlState::GeneralError, "HY000"),
            (SqlState::MemoryAllocationError, "HY001"),
            (SqlState::FunctionSequenceError, "HY010"),
            (SqlState::OptionalFeatureNotImplemented, "HYC00"),
            (SqlState::TimeoutExpired, "HYT00"),
            (SqlState::DriverDoesNotSupportThisFunction, "IM001"),
        ];

        for (state, expected_code) in error_states {
            assert!(state.is_error(), "Expected {state:?} to be an error");
            assert!(!state.is_success(), "Expected {state:?} to not be success");
            assert!(
                !state.is_warning(),
                "Expected {state:?} to not be a warning"
            );
            assert_eq!(state.as_str(), expected_code);
        }
    }
}
