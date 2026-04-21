const DISABLE_ENV: &str = "SNOWFLAKE_DISABLE_PLATFORM_DETECTION";

pub async fn detect_platforms() -> Vec<String> {
    if std::env::var(DISABLE_ENV)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return vec!["disabled".to_string()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_disabled_when_env_flag_true() {
        temp_env::async_with_vars([(DISABLE_ENV, Some("true"))], async {
            assert_eq!(detect_platforms().await, vec!["disabled".to_string()]);
        })
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_env_flag_false() {
        temp_env::async_with_vars([(DISABLE_ENV, Some("false"))], async {
            let platforms = detect_platforms().await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_env_flag_unset() {
        temp_env::async_with_vars([(DISABLE_ENV, None::<&str>)], async {
            let platforms = detect_platforms().await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }
}
