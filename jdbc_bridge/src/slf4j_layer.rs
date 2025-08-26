use jni::JavaVM;
use std::fmt::Debug;
use tracing::{Event, Level, Subscriber, field::Field};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

pub(crate) struct SLF4JLayer {
    jvm: *mut jni::sys::JavaVM,
}

impl SLF4JLayer {
    pub fn new(jvm: *mut jni::sys::JavaVM) -> Self {
        Self { jvm }
    }
}

impl<S> Layer<S> for SLF4JLayer
where
    S: Subscriber,
{
    /*
        This layer is used to log events to SLF4J.
        TODO:
          - Pass more information to SLF4J: file, line, etc.
    */
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
        // Format the log message
        let log_msg = format!("[{filename}:{line}] {message}");

        // Call SLF4J logger through JNI
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

        // Get org.slf4j.LoggerFactory class
        let logger_factory = env.find_class("org/slf4j/LoggerFactory").unwrap();
        let logger_name = env.new_string("com.snowflake.jdbc.CoreLogger").unwrap();
        // Get logger for our class
        let logger = env
            .call_static_method(
                logger_factory,
                "getLogger",
                "(Ljava/lang/String;)Lorg/slf4j/Logger;",
                &[(&logger_name).into()],
            )
            .unwrap()
            .l()
            .unwrap();

        let java_log_msg = env.new_string(log_msg).unwrap();

        // Call appropriate log level method
        env.call_method(
            logger,
            level_str.to_lowercase().as_str(),
            "(Ljava/lang/String;)V",
            &[(&java_log_msg).into()],
        )
        .unwrap();
    }
}

/*
TODO: Not sure if this will work,
  - jvm needs to be thread-safe
  - *mut jni::sys::JavaVM is not Send or Sync
*/

unsafe impl Send for SLF4JLayer {}
unsafe impl Sync for SLF4JLayer {}
