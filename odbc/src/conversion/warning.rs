pub type Warnings = Vec<Warning>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    StringDataTruncated,
    NumericValueTruncated,
}
