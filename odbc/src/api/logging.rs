use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use snafu::{Location, OptionExt, ResultExt, Snafu};
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::Layer as SubscriberLayer;
use tracing_subscriber::{Registry, reload};

type BoxedLayer = Box<dyn SubscriberLayer<Registry> + Send + Sync>;
type InnerLayer = Option<BoxedLayer>;
type ReloadHandle = reload::Handle<InnerLayer, Registry>;

static LOG_HANDLE: OnceLock<ReloadHandle> = OnceLock::new();

const LOG_FILE_NAME: &str = "odbc.log";

/// Default total file count (active + backups) when `LogFileSize` is set but
/// `LogFileCount` is omitted.
const DEFAULT_LOG_FILE_COUNT: u32 = 2;
const BYTES_PER_MB: u64 = 1024 * 1024;

pub(crate) struct OdbcLogConfig {
    pub log_path: Option<PathBuf>,
    pub log_level: Option<LevelFilter>,
    pub log_file_size_mb: Option<u64>,
    pub log_file_count: Option<u32>,
}

#[derive(Snafu, Debug)]
pub(crate) enum LoggingError {
    #[snafu(display("Failed to create log file"))]
    FileCreation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to reload logging layer"))]
    Reload {
        source: reload::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Logging reload handle not initialized"))]
    HandleNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Creates a reloadable logging layer that starts with no file output.
///
/// The returned layer should be passed to `sf_core::logging::init_logging` as
/// the `extra_layer`. The reload handle is stored in a process-wide static for
/// later use by [`reconfigure_logging`].
pub(crate) fn create_reload_layer() -> reload::Layer<InnerLayer, Registry> {
    let (layer, handle) = reload::Layer::new(None);
    if LOG_HANDLE.set(handle).is_err() {
        panic!("ODBC reload handle already initialized");
    }
    layer
}

/// Reconfigures the ODBC file logging layer based on DSN parameters.
///
/// When `log_path` is `Some`, creates an `fmt` layer writing to
/// `<log_path>/odbc.log` filtered at the requested level (defaulting to INFO).
/// When `log_file_size_mb` is set, uses a [`RollingFileWriter`] that rotates
/// the log file once it exceeds the size threshold, keeping up to
/// `log_file_count` total files (active + backups).
/// When `log_path` is `None`, disables file logging.
pub(crate) fn reconfigure_logging(config: &OdbcLogConfig) -> Result<(), LoggingError> {
    let handle = LOG_HANDLE.get().context(HandleNotInitializedSnafu)?;

    let new_layer: InnerLayer = match &config.log_path {
        Some(path) => {
            let level = config.log_level.unwrap_or(LevelFilter::INFO);

            if let Some(max_size_mb) = config.log_file_size_mb {
                let max_files = config.log_file_count.unwrap_or(DEFAULT_LOG_FILE_COUNT);
                let writer = RollingFileWriter::new(path.clone(), max_size_mb, max_files)
                    .context(FileCreationSnafu)?;
                Some(Box::new(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(Mutex::new(writer))
                        .with_filter(level),
                ))
            } else {
                let appender = make_appender(path).context(FileCreationSnafu)?;
                Some(Box::new(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(appender)
                        .with_filter(level),
                ))
            }
        }
        None => None,
    };

    handle.reload(new_layer).context(ReloadSnafu)?;
    Ok(())
}

/// Creates a non-rotating [`RollingFileAppender`] writing to
/// `<directory>/odbc.log`.
fn make_appender(directory: &Path) -> io::Result<RollingFileAppender> {
    RollingFileAppender::builder()
        .rotation(Rotation::NEVER)
        .filename_prefix(LOG_FILE_NAME)
        .build(directory)
        .map_err(io::Error::other)
}

/// Appends `.{index}` to a base path to form a rotated log file name
/// (e.g. `odbc.log` → `odbc.log.1`).
fn rotated_path(base: &Path, index: u32) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(format!(".{index}"));
    PathBuf::from(name)
}

/// A size-based rolling wrapper around [`RollingFileAppender`].
///
/// Tracks the number of bytes written and, once the configured size threshold
/// is exceeded, rotates the underlying file: the active file is renamed to
/// `.1`, older backups shift (`.1` → `.2`, …), and the oldest backup beyond
/// `max_files` is deleted.  A fresh [`RollingFileAppender`] is then created for
/// the new active file.
///
/// `max_files` is the *total* number of files on disk (active + backups).
struct RollingFileWriter {
    appender: RollingFileAppender,
    directory: PathBuf,
    log_file_path: PathBuf,
    current_size: u64,
    max_size_bytes: u64,
    max_files: u32,
}

impl RollingFileWriter {
    fn new(directory: PathBuf, max_size_mb: u64, max_files: u32) -> io::Result<Self> {
        let log_file_path = directory.join(LOG_FILE_NAME);
        let current_size = std::fs::metadata(&log_file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let appender = make_appender(&directory)?;

        Ok(Self {
            appender,
            directory,
            log_file_path,
            current_size,
            max_size_bytes: max_size_mb.saturating_mul(BYTES_PER_MB),
            max_files,
        })
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.appender.flush()?;

        let max_backups = self.max_files.saturating_sub(1);

        if max_backups > 0 {
            let _ = std::fs::remove_file(rotated_path(&self.log_file_path, max_backups));

            for i in (1..max_backups).rev() {
                let _ = std::fs::rename(
                    rotated_path(&self.log_file_path, i),
                    rotated_path(&self.log_file_path, i + 1),
                );
            }

            let _ = std::fs::rename(&self.log_file_path, rotated_path(&self.log_file_path, 1));
        } else {
            // No backups to keep — remove the active file so the new appender
            // starts fresh rather than appending to the old content.
            let _ = std::fs::remove_file(&self.log_file_path);
        }

        self.appender = make_appender(&self.directory)?;
        self.current_size = 0;
        Ok(())
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.max_size_bytes > 0
            && self.current_size > 0
            && self.current_size + buf.len() as u64 > self.max_size_bytes
        {
            self.rotate()?;
        }
        let n = self.appender.write(buf)?;
        self.current_size += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.appender.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_path_appends_index() {
        let base = PathBuf::from("/tmp/logs/odbc.log");
        assert_eq!(
            rotated_path(&base, 1),
            PathBuf::from("/tmp/logs/odbc.log.1")
        );
        assert_eq!(
            rotated_path(&base, 3),
            PathBuf::from("/tmp/logs/odbc.log.3")
        );
    }

    fn make_temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("odbc_log_test_{}_{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rolling_writer_creates_new_file() {
        let dir = make_temp_dir();
        let log = dir.join(LOG_FILE_NAME);
        let mut w = RollingFileWriter::new(dir.clone(), 1, 3).unwrap();
        w.write_all(b"hello").unwrap();
        w.flush().unwrap();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rolling_writer_appends_to_existing_file() {
        let dir = make_temp_dir();
        let log = dir.join(LOG_FILE_NAME);
        std::fs::write(&log, "existing\n").unwrap();

        let mut w = RollingFileWriter::new(dir.clone(), 1, 3).unwrap();
        assert_eq!(w.current_size, 9);
        w.write_all(b"more\n").unwrap();
        w.flush().unwrap();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "existing\nmore\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rolling_writer_rotates_on_size_threshold() {
        let dir = make_temp_dir();
        let log = dir.join(LOG_FILE_NAME);

        // 10-byte max, 3 total files (active + 2 backups)
        let mut w = RollingFileWriter::new(dir.clone(), 0, 3).unwrap();
        w.max_size_bytes = 10;

        w.write_all(b"aaaaaaaaaa").unwrap(); // 10 bytes, no rotation yet
        w.write_all(b"bbbb").unwrap(); // exceeds 10 → rotate first
        w.flush().unwrap();

        assert_eq!(std::fs::read_to_string(&log).unwrap(), "bbbb");
        assert_eq!(
            std::fs::read_to_string(dir.join("odbc.log.1")).unwrap(),
            "aaaaaaaaaa"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rolling_writer_shifts_backups_and_deletes_oldest() {
        let dir = make_temp_dir();
        let log = dir.join(LOG_FILE_NAME);

        // max_files=3 → at most 2 backups (.1, .2)
        let mut w = RollingFileWriter::new(dir.clone(), 0, 3).unwrap();
        w.max_size_bytes = 5;

        w.write_all(b"11111").unwrap();
        w.write_all(b"22222").unwrap(); // triggers rotation 1
        w.write_all(b"33333").unwrap(); // triggers rotation 2
        w.write_all(b"44444").unwrap(); // triggers rotation 3, .2 should be deleted
        w.flush().unwrap();

        assert_eq!(std::fs::read_to_string(&log).unwrap(), "44444");
        assert_eq!(
            std::fs::read_to_string(dir.join("odbc.log.1")).unwrap(),
            "33333"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("odbc.log.2")).unwrap(),
            "22222"
        );
        assert!(!dir.join("odbc.log.3").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rolling_writer_max_files_one_truncates_without_backups() {
        let dir = make_temp_dir();
        let log = dir.join(LOG_FILE_NAME);

        let mut w = RollingFileWriter::new(dir.clone(), 0, 1).unwrap();
        w.max_size_bytes = 5;

        w.write_all(b"aaaaa").unwrap();
        w.write_all(b"bbb").unwrap(); // triggers rotation, but max_backups=0
        w.flush().unwrap();

        assert_eq!(std::fs::read_to_string(&log).unwrap(), "bbb");
        assert!(!dir.join("odbc.log.1").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rolling_writer_zero_max_size_never_rotates() {
        let dir = make_temp_dir();

        let mut w = RollingFileWriter::new(dir.clone(), 0, 3).unwrap();
        // max_size_bytes stays 0

        w.write_all(b"a".repeat(100).as_slice()).unwrap();
        w.write_all(b"b".repeat(100).as_slice()).unwrap();
        w.flush().unwrap();

        assert!(!dir.join("odbc.log.1").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
