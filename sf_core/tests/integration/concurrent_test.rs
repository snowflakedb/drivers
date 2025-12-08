use crate::common::test_utils::*;
use std::thread;

#[test]
#[ignore] // Requires Snowflake credentials
fn test_concurrent_connections() {
    let handles: Vec<_> = (0..5)
        .map(|i| {
            thread::spawn(move || {
                let client = SnowflakeTestClient::connect_with_default_auth();
                let _result = client.execute_query(&format!("SELECT {} as thread_id", i));
                i
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
}

#[test]
#[ignore] // Requires Snowflake credentials
fn test_connection_pool_stress() {
    let handles: Vec<_> = (0..20)
        .map(|i| {
            thread::spawn(move || {
                let client = SnowflakeTestClient::connect_with_default_auth();

                for j in 0..5 {
                    let _result = client
                        .execute_query(&format!("SELECT {} as thread_id, {} as query_num", i, j));
                }

                i
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.len(), 20);
}
