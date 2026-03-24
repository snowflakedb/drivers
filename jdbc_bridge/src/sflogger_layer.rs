use jni::JavaVM;
use jni::objects::JValue;
use std::fmt::Debug;
use tracing::{Event, Level, Subscriber, field::Field};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

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
        let mut message = String::new();
        event.record(&mut |field: &Field, value: &dyn Debug| {
            if field.name() == "message" {
                message.push_str(&format!("{value:?}"));
            } else {
                let name = field.name();
                message.push_str(&format!(" {name}={value:?}"));
            }
        });
        let line = event.metadata().line().unwrap_or(0);
        let filename = event.metadata().file().unwrap_or("unknown");
        let level = event.metadata().level();
        let level_str = match *level {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };
        let log_msg = format!("[{filename}:{line}] {message}");

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

        let logger_factory = env
            .find_class("net/snowflake/client/internal/log/SFLoggerFactory")
            .unwrap();
        let logger_name = env.new_string("com.snowflake.jdbc.CoreLogger").unwrap();
        let logger = env
            .call_static_method(
                logger_factory,
                "getLogger",
                "(Ljava/lang/String;)Lnet/snowflake/client/internal/log/SFLogger;",
                &[(&logger_name).into()],
            )
            .unwrap()
            .l()
            .unwrap();

        let java_log_msg = env.new_string(log_msg).unwrap();

        env.call_method(
            logger,
            level_str,
            "(Ljava/lang/String;Z)V",
            &[(&java_log_msg).into(), JValue::Bool(1)],
        )
        .unwrap();
    }
}

unsafe impl Send for SFLoggerLayer {}
unsafe impl Sync for SFLoggerLayer {}
