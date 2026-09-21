use sf_core::config::param_registry::Deprecation;

pub type Warnings = Vec<Warning>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    StringDataTruncated,
    NumericValueTruncated,
    RowError,
    OptionValueChanged,
    /// Soft failure during disconnect teardown (SQLSTATE 01002).
    DisconnectError,
    /// Connection-string keywords the driver does not recognize (SQLSTATE
    /// 01S00, native error 17). The connection still opens; the keys are
    /// carried so the diagnostic can name them, matching the 3.x driver.
    UnrecognizedConnectionStringKeys(Vec<String>),
    /// Legacy connection-string key the driver still accepts (SQLSTATE 01000).
    /// `deprecation` supplies the text: the successor key, or guidance for a
    /// key that is accepted and not applied.
    DeprecatedParameter {
        parameter: String,
        deprecation: Deprecation,
    },
}
