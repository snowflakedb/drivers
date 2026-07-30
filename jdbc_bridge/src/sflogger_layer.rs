use jni::JavaVM;
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
    jvm: *mut jni::sys::JavaVM,
}

impl SFLoggerLayer {
    pub fn new(jvm: *mut jni::sys::JavaVM) -> Self {
        Self { jvm }
    }
}

impl<S> Layer<S> for SFLoggerLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let fields = sf_core::logging::normalize_event(event);
        let (logger_name, log_msg) = delivery_fields(&fields);

        let level_str = match fields.level {
            0 => "error",
            1 => "warn",
            2 => "info",
            _ => "debug",
        };

        let jvm = match unsafe { JavaVM::from_raw(self.jvm) } {
            Ok(jvm) => jvm,
            Err(e) => {
                eprintln!("Failed to get JavaVM: {e:?}");
                return;
            }
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                eprintln!("Failed to attach current thread: {e:?}");
                return;
            }
        };

        let logger_factory =
            match env.find_class("net/snowflake/client/internal/log/SFLoggerFactory") {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to find SFLoggerFactory class: {e:?}");
                    let _ = env.exception_clear();
                    return;
                }
            };
        let logger_name = match env.new_string(&logger_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create logger name string: {e:?}");
                let _ = env.exception_clear();
                return;
            }
        };
        // Delivery must use a plain logger (JUL or SLF4J), not a CoreLogger, or the record
        // would round-trip through core again and loop forever.
        let logger = match env
            .call_static_method(
                logger_factory,
                "getDeliveryLogger",
                "(Ljava/lang/String;)Lnet/snowflake/client/internal/log/SFLogger;",
                &[(&logger_name).into()],
            )
            .and_then(|v| v.l())
        {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to get delivery SFLogger instance: {e:?}");
                let _ = env.exception_clear();
                return;
            }
        };

        let java_log_msg = match env.new_string(log_msg) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create log message string: {e:?}");
                let _ = env.exception_clear();
                return;
            }
        };

        if let Err(e) = env.call_method(
            logger,
            level_str,
            "(Ljava/lang/String;)V",
            &[(&java_log_msg).into()],
        ) {
            eprintln!("Failed to call SFLogger.{level_str}: {e:?}");
            let _ = env.exception_clear();
        }
    }
}

unsafe impl Send for SFLoggerLayer {}
unsafe impl Sync for SFLoggerLayer {}

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
}
