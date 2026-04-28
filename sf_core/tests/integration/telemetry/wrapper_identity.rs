use sf_core::apis::database_driver_v1::DatabaseDriverV1;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;

fn test_identity() -> WrapperIdentity {
    WrapperIdentity {
        driver_name: "snowflake-connector-python".to_string(),
        driver_version: "3.5.0".to_string(),
        language_runtime: "CPython".to_string(),
        language_version: "3.12.1".to_string(),
        language_compiler: Some("GCC 13.2.0".to_string()),
    }
}

#[tokio::test]
async fn set_then_get_roundtrips() {
    let driver = DatabaseDriverV1::new();
    let handle = driver.connection_new();

    driver
        .set_wrapper_identity(handle, test_identity())
        .await
        .expect("set_wrapper_identity should succeed");

    let identity = driver
        .get_wrapper_identity(handle)
        .await
        .expect("get_wrapper_identity should succeed")
        .expect("identity should be Some after set");

    assert_eq!(identity.driver_name, "snowflake-connector-python");
    assert_eq!(identity.driver_version, "3.5.0");
    assert_eq!(identity.language_runtime, "CPython");
    assert_eq!(identity.language_version, "3.12.1");
    assert_eq!(identity.language_compiler, Some("GCC 13.2.0".to_string()));
}

#[tokio::test]
async fn get_returns_none_before_set() {
    let driver = DatabaseDriverV1::new();
    let handle = driver.connection_new();

    let identity = driver
        .get_wrapper_identity(handle)
        .await
        .expect("get_wrapper_identity should succeed");

    assert!(identity.is_none());
}

#[tokio::test]
async fn set_with_none_compiler() {
    let driver = DatabaseDriverV1::new();
    let handle = driver.connection_new();

    let identity = WrapperIdentity {
        language_compiler: None,
        ..test_identity()
    };

    driver
        .set_wrapper_identity(handle, identity)
        .await
        .expect("set_wrapper_identity should succeed");

    let stored = driver.get_wrapper_identity(handle).await.unwrap().unwrap();

    assert!(stored.language_compiler.is_none());
}

#[tokio::test]
async fn set_twice_overwrites() {
    let driver = DatabaseDriverV1::new();
    let handle = driver.connection_new();

    let first = WrapperIdentity {
        driver_version: "1.0.0".to_string(),
        ..test_identity()
    };
    let second = WrapperIdentity {
        driver_version: "2.0.0".to_string(),
        ..test_identity()
    };

    driver
        .set_wrapper_identity(handle, first)
        .await
        .expect("first set should succeed");
    driver
        .set_wrapper_identity(handle, second)
        .await
        .expect("second set should succeed (overwrite)");

    let stored = driver.get_wrapper_identity(handle).await.unwrap().unwrap();

    assert_eq!(stored.driver_version, "2.0.0");
}

#[tokio::test]
async fn set_with_invalid_handle_returns_error() {
    use sf_core::apis::database_driver_v1::Handle;

    let driver = DatabaseDriverV1::new();
    let bad_handle = Handle { id: 9999, magic: 0 };

    let result = driver
        .set_wrapper_identity(bad_handle, test_identity())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_with_invalid_handle_returns_error() {
    use sf_core::apis::database_driver_v1::Handle;

    let driver = DatabaseDriverV1::new();
    let bad_handle = Handle { id: 9999, magic: 0 };

    let result = driver.get_wrapper_identity(bad_handle).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn identity_is_per_connection() {
    let driver = DatabaseDriverV1::new();
    let handle_a = driver.connection_new();
    let handle_b = driver.connection_new();

    let identity_a = WrapperIdentity {
        driver_name: "python".to_string(),
        ..test_identity()
    };
    let identity_b = WrapperIdentity {
        driver_name: "nodejs".to_string(),
        ..test_identity()
    };

    driver
        .set_wrapper_identity(handle_a, identity_a)
        .await
        .unwrap();
    driver
        .set_wrapper_identity(handle_b, identity_b)
        .await
        .unwrap();

    let stored_a = driver
        .get_wrapper_identity(handle_a)
        .await
        .unwrap()
        .unwrap();
    let stored_b = driver
        .get_wrapper_identity(handle_b)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(stored_a.driver_name, "python");
    assert_eq!(stored_b.driver_name, "nodejs");
}
