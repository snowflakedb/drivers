/// Generates a unique table name by appending a UUID v4 (sourced from OS
/// entropy, i.e. /dev/urandom) to the base name. Prevents collisions when
/// concurrent CI runs share the same Snowflake schema.
pub fn unique_table_name(base: &str) -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("{base}_{id}")
}

/// RAII guard that executes a cleanup function on drop, ensuring table cleanup
/// even if the test panics. Swallows panics in the cleanup to avoid double-panic
/// aborts during stack unwinding.
pub struct TableCleanupGuard<'a> {
    table_name: String,
    drop_fn: Box<dyn Fn(&str) + 'a>,
}

impl<'a> TableCleanupGuard<'a> {
    pub fn new(table_name: String, drop_fn: impl Fn(&str) + 'a) -> Self {
        Self {
            table_name,
            drop_fn: Box::new(drop_fn),
        }
    }
}

impl Drop for TableCleanupGuard<'_> {
    fn drop(&mut self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.drop_fn)(&self.table_name);
        }));
    }
}
