pub type Warnings = Vec<Warning>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    StringDataTruncated,
    NumericValueTruncated,
    RowError,
    OptionValueChanged,
    /// Soft failure during disconnect teardown (SQLSTATE 01002).
    DisconnectError,
}
