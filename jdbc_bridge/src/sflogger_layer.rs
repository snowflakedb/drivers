use std::sync::OnceLock;

use jni::objects::{GlobalRef, JObject};
use jni::{JNIEnv, JavaVM};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Logger name for core-originated events (those with no wrapper logger name).
const CORE_LOGGER_NAME: &str = "net.snowflake.client.CoreLogger";

// Wrapper round-trip events carry a fully formatted message; deliver it verbatim
// to the originating logger. Core-originated events have no wrapper logger name,
// so prefix with source location before handing to the delivery logger.
fn delivery_fields(fields: &sf_core::logging::NormalizedEvent) -> (String, String) {
    if !fields.logger_name.is_empty() {
        (fields.logger_name.clone(), fields.message.clone())
    } else {
        (
            CORE_LOGGER_NAME.to_owned(),
            format!("[{}:{}] {}", fields.file, fields.line, fields.message),
        )
    }
}

pub(crate) struct SFLoggerLayer {
    jvm: JavaVM,
    core_logger: OnceLock<GlobalRef>,
}

impl SFLoggerLayer {
    pub fn new(jvm: *mut jni::sys::JavaVM) -> jni::errors::Result<Self> {
        Ok(Self {
            jvm: unsafe { JavaVM::from_raw(jvm)? },
            core_logger: OnceLock::new(),
        })
    }

    fn delivery_logger<'local>(
        env: &mut JNIEnv<'local>,
        logger_name: &str,
    ) -> jni::errors::Result<JObject<'local>> {
        let logger_factory = env.find_class("net/snowflake/client/internal/log/SFLoggerFactory")?;
        let logger_name = env.new_string(logger_name)?;
        env.call_static_method(
            logger_factory,
            "getDeliveryLogger",
            "(Ljava/lang/String;)Lnet/snowflake/client/internal/log/SFLogger;",
            &[(&logger_name).into()],
        )?
        .l()
    }

    fn core_logger(&self, env: &mut JNIEnv<'_>) -> jni::errors::Result<GlobalRef> {
        if let Some(logger) = self.core_logger.get() {
            return Ok(logger.clone());
        }

        let local = Self::delivery_logger(env, CORE_LOGGER_NAME)?;
        let logger = env.new_global_ref(local)?;
        let _ = self.core_logger.set(logger.clone());
        Ok(self.core_logger.get().cloned().unwrap_or(logger))
    }

    fn core_debug_enabled(&self, env: &mut JNIEnv<'_>) -> jni::errors::Result<bool> {
        let logger = self.core_logger(env)?;
        env.call_method(logger.as_obj(), "isDebugEnabled", "()Z", &[])?
            .z()
    }

    fn deliver(
        &self,
        env: &mut JNIEnv<'_>,
        fields: &sf_core::logging::NormalizedEvent,
        logger_name: &str,
        log_msg: &str,
    ) -> jni::errors::Result<()> {
        let level_str = match fields.level {
            0 => "error",
            1 => "warn",
            2 => "info",
            _ => "debug",
        };
        let java_log_msg = env.new_string(log_msg)?;

        if fields.logger_name.is_empty() {
            let logger = self.core_logger(env)?;
            env.call_method(
                logger.as_obj(),
                level_str,
                "(Ljava/lang/String;)V",
                &[(&java_log_msg).into()],
            )?;
        } else {
            let logger = Self::delivery_logger(env, logger_name)?;
            env.call_method(
                logger,
                level_str,
                "(Ljava/lang/String;)V",
                &[(&java_log_msg).into()],
            )?;
        }
        Ok(())
    }
}

fn should_check_core_debug_enabled(target: &str, level: &tracing::Level) -> bool {
    target != sf_core::logging::WRAPPER_TARGET
        && matches!(*level, tracing::Level::DEBUG | tracing::Level::TRACE)
}

impl<S> Layer<S> for SFLoggerLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut env = match self.jvm.attach_current_thread_as_daemon() {
            Ok(env) => env,
            Err(e) => {
                eprintln!("Failed to attach current thread: {e:?}");
                return;
            }
        };

        if let Err(e) = env.with_local_frame(8, |env| {
            let metadata = event.metadata();
            if should_check_core_debug_enabled(metadata.target(), metadata.level())
                && !self.core_debug_enabled(env)?
            {
                return Ok(());
            }

            let fields = sf_core::logging::normalize_event(event);
            let (logger_name, log_msg) = delivery_fields(&fields);
            self.deliver(env, &fields, &logger_name, &log_msg)
        }) {
            eprintln!("Failed to deliver log event: {e:?}");
            let _ = env.exception_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::logging::NormalizedEvent;

    fn sample_fields() -> NormalizedEvent {
        NormalizedEvent {
            level: 2,
            message: "hello".to_owned(),
            file: "foo.rs".to_owned(),
            line: 42,
            function: "bar".to_owned(),
            logger_name: String::new(),
        }
    }

    #[test]
    fn should_prefix_core_event_with_source_location() {
        let fields = sample_fields();
        let (logger_name, log_msg) = delivery_fields(&fields);
        assert_eq!(logger_name, CORE_LOGGER_NAME);
        assert_eq!(log_msg, "[foo.rs:42] hello");
    }

    #[test]
    fn should_deliver_wrapper_round_trip_verbatim() {
        let mut fields = sample_fields();
        fields.logger_name = "net.snowflake.client.Foo".to_owned();
        fields.message = "formatted message".to_owned();

        let (logger_name, log_msg) = delivery_fields(&fields);

        assert_eq!(logger_name, "net.snowflake.client.Foo");
        assert_eq!(log_msg, "formatted message");
    }

    #[test]
    fn should_gate_core_debug_and_trace_events() {
        assert!(should_check_core_debug_enabled(
            "sf_core::file_manager",
            &tracing::Level::DEBUG
        ));
        assert!(should_check_core_debug_enabled(
            "aws_smithy_runtime",
            &tracing::Level::TRACE
        ));
    }

    #[test]
    fn should_not_gate_wrapper_or_info_events() {
        assert!(!should_check_core_debug_enabled(
            sf_core::logging::WRAPPER_TARGET,
            &tracing::Level::DEBUG
        ));
        assert!(!should_check_core_debug_enabled(
            "sf_core::file_manager",
            &tracing::Level::INFO
        ));
    }
}
