use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing_subscriber::fmt::MakeWriter;

/// A thread-safe, size-based rotating file writer for use with `tracing_subscriber`.
///
/// When the active log file reaches `max_file_size` bytes, it is rotated:
/// the current file is renamed to `{base_path}.1`, any existing `.1` becomes `.2`,
/// and so on up to `max_file_count` rotated files. Files beyond the count are deleted.
pub struct RollingFileWriter {
    state: Mutex<WriterState>,
    base_path: PathBuf,
    max_file_size: u64,
    max_file_count: usize,
}

struct WriterState {
    file: File,
    current_size: u64,
}

impl RollingFileWriter {
    /// Opens (or creates) the log file at `base_path` in append mode.
    ///
    /// * `base_path` -- full path to the active log file (e.g. `/var/log/driver.log`).
    /// * `max_file_size` -- byte threshold that triggers rotation.
    /// * `max_file_count` -- number of rotated files (`.1` .. `.N`) to retain.
    pub fn new(
        base_path: impl Into<PathBuf>,
        max_file_size: u64,
        max_file_count: usize,
    ) -> io::Result<Self> {
        let base_path = base_path.into();

        if let Some(parent) = base_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }

        let (file, current_size) = open_and_measure(&base_path)?;

        Ok(Self {
            state: Mutex::new(WriterState { file, current_size }),
            base_path,
            max_file_size,
            max_file_count,
        })
    }

    fn rotate(&self, state: &mut WriterState) -> io::Result<()> {
        state.file.flush()?;

        // Delete the oldest rotated file if it would be pushed beyond max_file_count.
        let oldest = rotated_path(&self.base_path, self.max_file_count);
        if oldest.exists() {
            fs::remove_file(&oldest)?;
        }

        // Shift each rotated file up by one index (high to low to avoid overwrites).
        for i in (1..self.max_file_count).rev() {
            let from = rotated_path(&self.base_path, i);
            let to = rotated_path(&self.base_path, i + 1);
            if from.exists() {
                fs::rename(&from, &to)?;
            }
        }

        // Move the active file to .1 (unless max_file_count is 0, just remove it).
        if self.max_file_count > 0 {
            fs::rename(&self.base_path, rotated_path(&self.base_path, 1))?;
        } else {
            fs::remove_file(&self.base_path)?;
        }

        let (file, current_size) = open_and_measure(&self.base_path)?;
        state.file = file;
        state.current_size = current_size;

        Ok(())
    }
}

fn open_and_measure(path: &Path) -> io::Result<(File, u64)> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let current_size = file.metadata()?.len();
    Ok((file, current_size))
}

fn rotated_path(base: &Path, index: usize) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(format!(".{index}"));
    PathBuf::from(name)
}

impl Write for &RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        if state.current_size >= self.max_file_size
            && state.current_size > 0
            && let Err(e) = self.rotate(&mut state)
        {
            eprintln!("Log file rotation failed: {e}");
        }

        let written = state.file.write(buf)?;
        state.current_size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.file.flush()
    }
}

impl<'a> MakeWriter<'a> for RollingFileWriter {
    type Writer = &'a Self;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn writer_in(dir: &Path, size: u64, count: usize) -> RollingFileWriter {
        RollingFileWriter::new(dir.join("test.log"), size, count).unwrap()
    }

    #[test]
    fn basic_write() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 1024, 3);

        let mut w = &writer;
        w.write_all(b"hello\n").unwrap();
        w.flush().unwrap();

        let content = fs::read_to_string(dir.path().join("test.log")).unwrap();
        assert_eq!(content, "hello\n");
    }

    #[test]
    fn rotates_when_size_exceeded() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 10, 3);

        let mut w = &writer;
        w.write_all(b"1234567890").unwrap(); // exactly 10 bytes -> at limit
        w.write_all(b"next line\n").unwrap(); // triggers rotation, then writes

        let rotated = fs::read_to_string(dir.path().join("test.log.1")).unwrap();
        assert_eq!(rotated, "1234567890");

        let active = fs::read_to_string(dir.path().join("test.log")).unwrap();
        assert_eq!(active, "next line\n");
    }

    #[test]
    fn shifts_rotated_files() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 5, 3);

        let mut w = &writer;
        w.write_all(b"aaaaa").unwrap(); // fills to limit
        w.write_all(b"bbbbb").unwrap(); // rotation 1: aaaaa -> .1
        w.write_all(b"ccccc").unwrap(); // rotation 2: bbbbb -> .1, aaaaa -> .2
        w.write_all(b"ddddd").unwrap(); // rotation 3: ccccc -> .1, bbbbb -> .2, aaaaa -> .3

        assert_eq!(
            fs::read_to_string(dir.path().join("test.log")).unwrap(),
            "ddddd"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("test.log.1")).unwrap(),
            "ccccc"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("test.log.2")).unwrap(),
            "bbbbb"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("test.log.3")).unwrap(),
            "aaaaa"
        );
    }

    #[test]
    fn drops_files_beyond_max_count() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 5, 2);

        let mut w = &writer;
        w.write_all(b"aaaaa").unwrap();
        w.write_all(b"bbbbb").unwrap(); // .1 = aaaaa
        w.write_all(b"ccccc").unwrap(); // .1 = bbbbb, .2 = aaaaa
        w.write_all(b"ddddd").unwrap(); // .1 = ccccc, .2 = bbbbb, aaaaa deleted

        assert_eq!(
            fs::read_to_string(dir.path().join("test.log")).unwrap(),
            "ddddd"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("test.log.1")).unwrap(),
            "ccccc"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("test.log.2")).unwrap(),
            "bbbbb"
        );
        assert!(!dir.path().join("test.log.3").exists());
    }

    #[test]
    fn zero_max_count_discards_rotated_files() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 5, 0);

        let mut w = &writer;
        w.write_all(b"aaaaa").unwrap();
        w.write_all(b"bbbbb").unwrap();

        assert_eq!(
            fs::read_to_string(dir.path().join("test.log")).unwrap(),
            "bbbbb"
        );
        assert!(!dir.path().join("test.log.1").exists());
    }

    #[test]
    fn resumes_existing_file() {
        let dir = TempDir::new().unwrap();
        let log_path = dir.path().join("test.log");
        fs::write(&log_path, b"existing").unwrap();

        let writer = RollingFileWriter::new(&log_path, 20, 2).unwrap();
        let mut w = &writer;
        w.write_all(b"_appended").unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, "existing_appended");
    }

    #[test]
    fn resumes_and_rotates_when_already_over_limit() {
        let dir = TempDir::new().unwrap();
        let log_path = dir.path().join("test.log");
        fs::write(&log_path, b"big content already here").unwrap();

        let writer = RollingFileWriter::new(&log_path, 10, 2).unwrap();
        let mut w = &writer;
        w.write_all(b"new").unwrap(); // should trigger rotation first

        let rotated = fs::read_to_string(dir.path().join("test.log.1")).unwrap();
        assert_eq!(rotated, "big content already here");

        let active = fs::read_to_string(&log_path).unwrap();
        assert_eq!(active, "new");
    }

    #[test]
    fn creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let deep_path = dir.path().join("a").join("b").join("c").join("test.log");

        let writer = RollingFileWriter::new(&deep_path, 1024, 2).unwrap();
        let mut w = &writer;
        w.write_all(b"hello").unwrap();

        assert_eq!(fs::read_to_string(&deep_path).unwrap(), "hello");
    }

    #[test]
    fn make_writer_returns_working_writer() {
        let dir = TempDir::new().unwrap();
        let writer = writer_in(dir.path(), 1024, 2);

        let mut w = MakeWriter::make_writer(&writer);
        w.write_all(b"via make_writer").unwrap();
        w.flush().unwrap();

        let content = fs::read_to_string(dir.path().join("test.log")).unwrap();
        assert_eq!(content, "via make_writer");
    }
}
