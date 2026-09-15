pub type Warnings = Vec<Warning>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    StringDataTruncated,
    NumericValueTruncated,
    RowError,
    OptionValueChanged,
    /// Soft failure during disconnect teardown (SQLSTATE 01002).
    DisconnectError,
    /// Legacy connection-string key accepted as an alias (SQLSTATE 01000).
    DeprecatedParameter {
        parameter: String,
        replacement: &'static str,
    },
}
