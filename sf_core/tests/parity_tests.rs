#[test]
fn async_facade_symbol_exists() {
    // Ensure the async-backed blocking facade symbol is exported.
    let _sym = sf_core::rest::snowflake::snowflake_query_async_style::<&str> as *const ();
}
