/// Generates a unique table name by appending a nanosecond timestamp to the
/// base name. Prevents collisions when concurrent CI runs share the same
/// Snowflake schema.
pub fn unique_table_name(base: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{base}_{nanos}")
}
