#![allow(clippy::result_large_err)]

use std::any::Any;
use std::ffi::{CStr, c_char, c_int};
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::{Arc, LazyLock};

use arrow::array::{RecordBatch, RecordBatchIterator, RecordBatchReader};
use arrow::datatypes::{Schema, SchemaRef};
use arrow::error::ArrowError;
use arrow::ffi::FFI_ArrowArray;
use arrow::ffi_stream::FFI_ArrowArrayStream;

use super::{internal_error, invalid_argument};
use crate::protobuf::generated::database_driver_v1::*;

struct CallerPtr<T>(NonNull<T>);

// SAFETY: a `CallerPtr` is the only handle the core holds to its target during one RPC, and the
// caller does not access the target until that RPC completes, so moving the pointer to another
// thread cannot introduce concurrent access. `T: Send` keeps this no broader than `&mut T: Send`.
unsafe impl<T: Send> Send for CallerPtr<T> {}

impl<T> CallerPtr<T> {
    fn required(value: Option<Vec<u8>>, field_name: &str) -> Result<Self, DriverException> {
        let value = value.ok_or_else(|| invalid_argument(format!("{field_name} is required")))?;
        Self::decode(&value, field_name)
    }

    fn decode(value: &[u8], field_name: &str) -> Result<Self, DriverException> {
        let bytes: [u8; 8] = value.try_into().map_err(|_| {
            invalid_argument(format!("{field_name} must be 8 bytes, got {}", value.len()))
        })?;
        let address = usize::try_from(u64::from_le_bytes(bytes))
            .map_err(|_| invalid_argument(format!("{field_name} does not fit in usize")))?;
        let ptr = NonNull::new(std::ptr::with_exposed_provenance_mut::<T>(address))
            .ok_or_else(|| invalid_argument(format!("{field_name} must not be null")))?;
        if !ptr.is_aligned() {
            return Err(invalid_argument(format!("{field_name} is not aligned")));
        }
        Ok(Self(ptr))
    }

    fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

impl CallerPtr<FFI_ArrowArrayStream> {
    fn new(out_stream: Option<ArrowArrayStreamPtr>) -> Result<Self, DriverException> {
        let slot = CallerPtr::<FFI_ArrowArrayStream>::required(
            out_stream.map(|ptr| ptr.value),
            "Out stream pointer",
        )?;
        // SAFETY: the caller guarantees the slot is an initialized `ArrowArrayStream` it does not
        // access until this call completes; `decode` checked it is non-null and aligned.
        if unsafe { (*slot.as_ptr()).release.is_some() } {
            return Err(invalid_argument(
                "Out stream pointer already holds a live stream",
            ));
        }
        Ok(slot)
    }

    fn fill(self, reader: Box<dyn RecordBatchReader + Send>) {
        let reader = GuardedReader {
            inner: reader,
            panicked: false,
        };
        // SAFETY: `new` checked that the slot is released, and the caller does not access it until
        // this call completes, so overwriting it without dropping its contents leaks nothing.
        unsafe {
            self.0
                .as_ptr()
                .write(FFI_ArrowArrayStream::new(Box::new(reader)))
        };
    }
}

type GetNext = unsafe extern "C" fn(*mut FFI_ArrowArrayStream, *mut FFI_ArrowArray) -> c_int;
type GetLastError = unsafe extern "C" fn(*mut FFI_ArrowArrayStream) -> *const c_char;
type Release = unsafe extern "C" fn(*mut FFI_ArrowArrayStream);

#[derive(Clone, Copy)]
struct StreamCallbacks {
    get_next: GetNext,
    get_last_error: GetLastError,
    release: Release,
}

static ARROW_STREAM_CALLBACKS: LazyLock<Option<StreamCallbacks>> = LazyLock::new(|| {
    let empty = RecordBatchIterator::new(
        std::iter::empty::<Result<RecordBatch, ArrowError>>(),
        Arc::new(Schema::empty()),
    );
    let stream = FFI_ArrowArrayStream::new(Box::new(empty));
    Some(StreamCallbacks {
        get_next: stream.get_next?,
        get_last_error: stream.get_last_error?,
        release: stream.release?,
    })
});

pub(super) async fn export_stream(
    out_stream: Option<ArrowArrayStreamPtr>,
    open: impl Future<Output = Result<Box<dyn RecordBatchReader + Send>, DriverException>>,
) -> Result<(), DriverException> {
    let slot = CallerPtr::<FFI_ArrowArrayStream>::new(out_stream)?;
    let reader = open.await?;
    slot.fill(reader);
    Ok(())
}

struct GuardedReader {
    inner: Box<dyn RecordBatchReader + Send>,
    panicked: bool,
}

impl Iterator for GuardedReader {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.panicked {
            return Some(Err(ArrowError::ExternalError(
                "Result set reader panicked while reading an earlier batch".into(),
            )));
        }
        match catch_unwind(AssertUnwindSafe(|| self.inner.next())) {
            Ok(item) => item.map(|result| result.map_err(without_nul)),
            Err(payload) => {
                self.panicked = true;
                let message = format!("Result set reader panicked: {}", panic_message(&*payload));
                Some(Err(ArrowError::ExternalError(replace_nul(&message).into())))
            }
        }
    }
}

impl RecordBatchReader for GuardedReader {
    fn schema(&self) -> SchemaRef {
        self.inner.schema()
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message")
}

fn replace_nul(text: &str) -> String {
    text.replace('\0', "\u{FFFD}")
}

fn without_nul(error: ArrowError) -> ArrowError {
    if !error.to_string().contains('\0') {
        return error;
    }
    match error {
        ArrowError::ExternalError(source) => {
            ArrowError::ExternalError(replace_nul(&source.to_string()).into())
        }
        ArrowError::IoError(description, source) => {
            ArrowError::IoError(replace_nul(&description), source)
        }
        ArrowError::NotYetImplemented(message) => {
            ArrowError::NotYetImplemented(replace_nul(&message))
        }
        ArrowError::MemoryError(message) => ArrowError::MemoryError(replace_nul(&message)),
        other => ArrowError::ExternalError(replace_nul(&other.to_string()).into()),
    }
}

pub(super) async fn get_next_batch(
    stream: Option<ArrowArrayStreamPtr>,
    out_array: Option<ArrowArrayPtr>,
) -> Result<ArrowStreamGetNextBatchResponse, DriverException> {
    let stream = CallerPtr::required(stream.map(|ptr| ptr.value), "Stream pointer")?;
    let out_array = CallerPtr::required(out_array.map(|ptr| ptr.value), "Out array pointer")?;
    let callbacks = ARROW_STREAM_CALLBACKS
        .ok_or_else(|| internal_error("Arrow stream callbacks are unavailable"))?;
    ensure_readable(&callbacks, &stream, &out_array)?;
    tokio::task::spawn_blocking(move || call_get_next(callbacks, stream, out_array))
        .await
        .map_err(|error| internal_error(format!("Arrow stream read task failed: {error}")))?
}

fn ensure_readable(
    callbacks: &StreamCallbacks,
    stream: &CallerPtr<FFI_ArrowArrayStream>,
    out_array: &CallerPtr<FFI_ArrowArray>,
) -> Result<(), DriverException> {
    // SAFETY: the caller guarantees `stream` points to an initialized `ArrowArrayStream` that
    // nothing else uses until this call completes; `decode` checked it is non-null and aligned.
    match unsafe { (*stream.as_ptr()).release } {
        None => return Err(invalid_argument("Stream is released")),
        Some(release) if !std::ptr::fn_addr_eq(release, callbacks.release) => {
            return Err(invalid_argument("Stream was not exported by the core"));
        }
        Some(_) => {}
    }
    // SAFETY: the caller guarantees `out_array` points to an initialized `ArrowArray` it neither
    // reads nor frees until this call completes; `decode` checked it is non-null and aligned.
    if !unsafe { &*out_array.as_ptr() }.is_released() {
        return Err(invalid_argument(
            "Out array pointer already holds a live array",
        ));
    }
    Ok(())
}

fn call_get_next(
    callbacks: StreamCallbacks,
    stream: CallerPtr<FFI_ArrowArrayStream>,
    out_array: CallerPtr<FFI_ArrowArray>,
) -> Result<ArrowStreamGetNextBatchResponse, DriverException> {
    let stream = stream.as_ptr();
    // SAFETY: the caller guarantees `stream` is a live stream the core exported, so `callbacks`
    // are the functions that operate on it, and nothing else uses it until this call completes;
    // the caller also guarantees `out_array` points to `ArrowArray` storage it neither reads nor
    // frees until then, and `decode` checked both are non-null and aligned.
    let status = unsafe { (callbacks.get_next)(stream, out_array.as_ptr()) };
    if status == 0 {
        return Ok(ArrowStreamGetNextBatchResponse {
            status,
            error_message: None,
        });
    }

    // SAFETY: the stream's last operation failed, which is when `get_last_error` may be called,
    // and nothing operates on the stream before the text is copied below.
    let text = unsafe { (callbacks.get_last_error)(stream) };
    let error_message = (!text.is_null()).then(|| {
        // SAFETY: a non-null `get_last_error` result is a NUL-terminated string that stays valid
        // until the next operation on the stream.
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    });
    // SAFETY: a failed `get_next` transfers no array to the caller, so `out_array` holds nothing
    // the caller owns, and overwriting it leaks nothing.
    unsafe { out_array.as_ptr().write(FFI_ArrowArray::empty()) };
    Ok(ArrowStreamGetNextBatchResponse {
        status,
        error_message,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::{c_char, c_int, c_void};
    use std::mem::ManuallyDrop;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::sync_channel;
    use std::sync::{Arc, OnceLock};
    use std::thread::{self, ThreadId};
    use std::time::Duration;

    use arrow::array::{
        Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray, StructArray,
    };
    use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
    use arrow::error::ArrowError;
    use arrow::ffi::{FFI_ArrowSchema, from_ffi_and_data_type};
    use arrow::ipc::writer::StreamWriter;
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    use proto_utils::ProtoError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::apis::database_driver_v1::ColumnMetadata;
    use crate::chunks::ChunkDownloadData;
    use crate::protobuf::apis::database_driver_v1::{DatabaseDriverClient, database_driver_client};
    use crate::rest::snowflake::query_response::RowsetData;

    const EIO: c_int = 5;
    const EINVAL: c_int = 22;
    const BOUND: Duration = Duration::from_secs(20);
    const MAX_READS: usize = 16;

    fn address_of<T>(ptr: *mut T) -> Vec<u8> {
        (ptr.expose_provenance() as u64).to_le_bytes().to_vec()
    }

    fn stream_address(stream: &mut FFI_ArrowArrayStream) -> Option<ArrowArrayStreamPtr> {
        Some(ArrowArrayStreamPtr {
            value: address_of(stream as *mut FFI_ArrowArrayStream),
        })
    }

    fn out_array_address(array: &mut FFI_ArrowArray) -> Option<ArrowArrayPtr> {
        Some(ArrowArrayPtr {
            value: address_of(array as *mut FFI_ArrowArray),
        })
    }

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![Field::new("n", DataType::Int64, false)]))
    }

    fn batch(values: Vec<i64>) -> RecordBatch {
        RecordBatch::try_new(schema(), vec![Arc::new(Int64Array::from(values))]).unwrap()
    }

    fn live_array() -> FFI_ArrowArray {
        FFI_ArrowArray::new(&StructArray::from(batch(vec![9])).to_data())
    }

    struct ScriptedReader<F>(F);

    impl<F> Iterator for ScriptedReader<F>
    where
        F: FnMut() -> Option<Result<RecordBatch, ArrowError>>,
    {
        type Item = Result<RecordBatch, ArrowError>;

        fn next(&mut self) -> Option<Self::Item> {
            self.0()
        }
    }

    impl<F> RecordBatchReader for ScriptedReader<F>
    where
        F: FnMut() -> Option<Result<RecordBatch, ArrowError>>,
    {
        fn schema(&self) -> SchemaRef {
            schema()
        }
    }

    fn core_stream(reader: impl RecordBatchReader + Send + 'static) -> FFI_ArrowArrayStream {
        let mut slot = FFI_ArrowArrayStream::empty();
        CallerPtr::<FFI_ArrowArrayStream>::new(stream_address(&mut slot))
            .unwrap()
            .fill(Box::new(reader));
        slot
    }

    fn exported(items: Vec<Result<RecordBatch, ArrowError>>) -> FFI_ArrowArrayStream {
        core_stream(RecordBatchIterator::new(items, schema()))
    }

    fn counting_reader(
        items: Vec<Result<RecordBatch, ArrowError>>,
    ) -> (impl RecordBatchReader + Send + 'static, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let mut items = items.into_iter();
        let reader = ScriptedReader(move || {
            counted.fetch_add(1, Ordering::Relaxed);
            items.next()
        });
        (reader, calls)
    }

    async fn next(
        stream: &mut FFI_ArrowArrayStream,
        out_array: &mut FFI_ArrowArray,
    ) -> Result<ArrowStreamGetNextBatchResponse, DriverException> {
        get_next_batch(stream_address(stream), out_array_address(out_array)).await
    }

    fn imported(array: FFI_ArrowArray) -> RecordBatch {
        // SAFETY: `array` was filled by the stream's `get_next` with a struct array of
        // `schema()`, and `from_ffi_and_data_type` takes ownership of it.
        let data =
            unsafe { from_ffi_and_data_type(array, DataType::Struct(schema().fields().clone())) }
                .unwrap();
        RecordBatch::from(StructArray::from(data))
    }

    #[derive(Default)]
    struct StubState {
        get_next_calls: AtomicUsize,
        get_next_status: c_int,
        last_error: Option<&'static CStr>,
        scribble_on_failure: bool,
    }

    /// # Safety
    ///
    /// `stream` was built by `stub_stream`, and its `StubState` outlives `'a`.
    unsafe fn stub_state<'a>(stream: *mut FFI_ArrowArrayStream) -> &'a StubState {
        // SAFETY: guaranteed by the caller.
        unsafe { &*(*stream).private_data.cast::<StubState>() }
    }

    unsafe extern "C" fn stub_get_next(
        stream: *mut FFI_ArrowArrayStream,
        out_array: *mut FFI_ArrowArray,
    ) -> c_int {
        // SAFETY: the stub callbacks are only installed on streams built by `stub_stream`.
        let state = unsafe { stub_state(stream) };
        state.get_next_calls.fetch_add(1, Ordering::Relaxed);
        if state.get_next_status != 0 && state.scribble_on_failure {
            // SAFETY: `out_array` is the caller's writable `ArrowArray` slot; the bytes written
            // are never released because the test keeps the slot in `ManuallyDrop`.
            unsafe { out_array.write_bytes(0xAB, 1) };
        }
        state.get_next_status
    }

    unsafe extern "C" fn stub_get_last_error(stream: *mut FFI_ArrowArrayStream) -> *const c_char {
        // SAFETY: the stub callbacks are only installed on streams built by `stub_stream`.
        unsafe { stub_state(stream) }
            .last_error
            .map_or(std::ptr::null(), CStr::as_ptr)
    }

    unsafe extern "C" fn stub_release(stream: *mut FFI_ArrowArrayStream) {
        // SAFETY: `release` is only called with the stream it belongs to.
        unsafe { (*stream).release = None };
    }

    fn stub_stream(state: &StubState) -> FFI_ArrowArrayStream {
        FFI_ArrowArrayStream {
            get_schema: None,
            get_next: Some(stub_get_next),
            get_last_error: Some(stub_get_last_error),
            release: Some(stub_release),
            private_data: state as *const StubState as *mut c_void,
        }
    }

    fn call_stub(
        stream: &mut FFI_ArrowArrayStream,
        out_array: &mut FFI_ArrowArray,
    ) -> Result<ArrowStreamGetNextBatchResponse, DriverException> {
        call_get_next(
            StreamCallbacks {
                get_next: stub_get_next,
                get_last_error: stub_get_last_error,
                release: stub_release,
            },
            CallerPtr::required(stream_address(stream).map(|p| p.value), "Stream pointer").unwrap(),
            CallerPtr::required(out_array_address(out_array).map(|p| p.value), "Out array")
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn reads_every_batch_then_reports_the_end_with_a_released_array() {
        let mut stream = exported(vec![Ok(batch(vec![1, 2])), Ok(batch(vec![3]))]);

        let mut first = FFI_ArrowArray::empty();
        assert_eq!(next(&mut stream, &mut first).await.unwrap().status, 0);
        assert_eq!(imported(first), batch(vec![1, 2]));

        let mut second = FFI_ArrowArray::empty();
        assert_eq!(next(&mut stream, &mut second).await.unwrap().status, 0);
        assert_eq!(imported(second), batch(vec![3]));

        let mut end = FFI_ArrowArray::empty();
        let response = next(&mut stream, &mut end).await.unwrap();
        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: 0,
                error_message: None,
            }
        );
        assert!(end.is_released());
    }

    #[tokio::test]
    async fn get_next_runs_off_the_runtime_thread() {
        let reader_thread = Arc::new(OnceLock::<ThreadId>::new());
        let recorded = Arc::clone(&reader_thread);
        let mut stream = core_stream(ScriptedReader(move || {
            recorded.get_or_init(|| thread::current().id());
            None
        }));

        let mut end = FFI_ArrowArray::empty();
        assert_eq!(next(&mut stream, &mut end).await.unwrap().status, 0);

        let reader_thread = *reader_thread.get().expect("get_next was not called");
        assert_ne!(reader_thread, thread::current().id());
    }

    #[tokio::test]
    async fn a_failing_batch_reports_einval_and_the_stream_error_text() {
        let mut stream = exported(vec![Err(ArrowError::ExternalError(
            "Failed to read chunk data".into(),
        ))]);

        let mut out_array = FFI_ArrowArray::empty();
        let response = next(&mut stream, &mut out_array).await.unwrap();

        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: EINVAL,
                error_message: Some("External error: Failed to read chunk data".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn a_batch_after_a_failure_reports_no_error_text() {
        let mut stream = exported(vec![
            Err(ArrowError::ExternalError("boom".into())),
            Ok(batch(vec![5])),
        ]);

        let mut failed = FFI_ArrowArray::empty();
        assert_eq!(next(&mut stream, &mut failed).await.unwrap().status, EINVAL);

        let mut out_array = FFI_ArrowArray::empty();
        let response = next(&mut stream, &mut out_array).await.unwrap();

        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: 0,
                error_message: None,
            }
        );
        assert_eq!(imported(out_array), batch(vec![5]));
    }

    #[test]
    fn a_failure_leaves_the_out_array_released_whatever_get_next_wrote() {
        let state = StubState {
            get_next_status: EIO,
            scribble_on_failure: true,
            ..StubState::default()
        };
        let mut stream = stub_stream(&state);
        let mut out_array = ManuallyDrop::new(FFI_ArrowArray::empty());

        let response = call_stub(&mut stream, &mut out_array).unwrap();

        assert_eq!(response.status, EIO);
        assert!(out_array.is_released());
    }

    #[test]
    fn a_failure_whose_get_last_error_returns_null_reports_only_the_status() {
        let state = StubState {
            get_next_status: EIO,
            ..StubState::default()
        };
        let mut stream = stub_stream(&state);
        let mut out_array = FFI_ArrowArray::empty();

        let response = call_stub(&mut stream, &mut out_array).unwrap();

        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: EIO,
                error_message: None,
            }
        );
        assert_eq!(state.get_next_calls.load(Ordering::Relaxed), 1);
    }

    unsafe extern "C" fn failing_get_next(
        _stream: *mut FFI_ArrowArrayStream,
        _out_array: *mut FFI_ArrowArray,
    ) -> c_int {
        EIO
    }

    unsafe extern "C" fn no_last_error(_stream: *mut FFI_ArrowArrayStream) -> *const c_char {
        std::ptr::null()
    }

    #[tokio::test]
    async fn the_core_reads_with_its_own_callbacks_not_the_ones_in_the_stream() {
        let mut stream = exported(vec![Ok(batch(vec![6]))]);
        stream.get_next = Some(failing_get_next);
        stream.get_last_error = Some(no_last_error);
        let mut out_array = FFI_ArrowArray::empty();

        let response = next(&mut stream, &mut out_array).await.unwrap();

        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: 0,
                error_message: None,
            }
        );
        assert_eq!(imported(out_array), batch(vec![6]));
    }

    #[tokio::test]
    async fn a_released_stream_is_rejected() {
        let mut stream = exported(vec![Ok(batch(vec![1]))]);
        let release = stream.release.unwrap();
        // SAFETY: `stream` is live and `release` is its own release callback.
        unsafe { release(&mut stream) };
        let mut out_array = FFI_ArrowArray::empty();

        let error = next(&mut stream, &mut out_array).await.unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(error.message, "Stream is released");
    }

    #[tokio::test]
    async fn a_live_out_array_is_rejected_and_left_as_it_is() {
        let (reader, reader_calls) = counting_reader(vec![Ok(batch(vec![1]))]);
        let mut stream = core_stream(reader);
        let mut out_array = live_array();

        let error = next(&mut stream, &mut out_array).await.unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(
            error.message,
            "Out array pointer already holds a live array"
        );
        assert!(!out_array.is_released());
        assert_eq!(reader_calls.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn missing_null_short_and_misaligned_pointers_are_rejected_before_get_next() {
        let (reader, reader_calls) = counting_reader(vec![Ok(batch(vec![1]))]);
        let mut stream = core_stream(reader);
        let mut out_array = FFI_ArrowArray::empty();
        let valid_stream = stream_address(&mut stream);
        let valid_array = out_array_address(&mut out_array);
        let raw_stream = |value: Vec<u8>| Some(ArrowArrayStreamPtr { value });
        let raw_array = |value: Vec<u8>| Some(ArrowArrayPtr { value });
        #[cfg_attr(not(target_pointer_width = "64"), allow(unused_mut))]
        let mut cases = vec![
            (None, valid_array.clone(), "Stream pointer is required"),
            (
                raw_stream(vec![0; 8]),
                valid_array.clone(),
                "Stream pointer must not be null",
            ),
            (
                raw_stream(vec![1; 4]),
                valid_array.clone(),
                "Stream pointer must be 8 bytes, got 4",
            ),
            (
                raw_stream(0x1001_u64.to_le_bytes().to_vec()),
                valid_array.clone(),
                "Stream pointer is not aligned",
            ),
            (valid_stream.clone(), None, "Out array pointer is required"),
            (
                valid_stream.clone(),
                raw_array(vec![0; 8]),
                "Out array pointer must not be null",
            ),
            (
                valid_stream.clone(),
                raw_array(vec![1; 9]),
                "Out array pointer must be 8 bytes, got 9",
            ),
            (
                valid_stream.clone(),
                raw_array(0x1001_u64.to_le_bytes().to_vec()),
                "Out array pointer is not aligned",
            ),
        ];
        #[cfg(target_pointer_width = "64")]
        cases.extend([
            (
                raw_stream(0x1004_u64.to_le_bytes().to_vec()),
                valid_array.clone(),
                "Stream pointer is not aligned",
            ),
            (
                valid_stream.clone(),
                raw_array(0x1004_u64.to_le_bytes().to_vec()),
                "Out array pointer is not aligned",
            ),
        ]);

        for (stream, out_array, expected) in cases {
            let error = get_next_batch(stream, out_array).await.unwrap_err();
            assert_eq!(
                error.kind,
                ErrorKind::InvalidArgument as i32,
                "case: {expected}"
            );
            assert_eq!(error.message, expected, "case: {expected}");
        }
        assert_eq!(reader_calls.load(Ordering::Relaxed), 0);
    }

    async fn open_probe(
        opened: &AtomicBool,
    ) -> Result<Box<dyn RecordBatchReader + Send>, DriverException> {
        opened.store(true, Ordering::Relaxed);
        Ok(Box::new(RecordBatchIterator::new(
            vec![Ok(batch(vec![8]))],
            schema(),
        )))
    }

    #[tokio::test]
    async fn a_missing_out_stream_is_rejected_without_opening_the_result() {
        let opened = AtomicBool::new(false);

        let error = export_stream(None, open_probe(&opened)).await.unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(error.message, "Out stream pointer is required");
        assert!(!opened.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn exporting_into_a_live_stream_is_rejected_without_opening_the_result() {
        let mut stream = exported(vec![Ok(batch(vec![7]))]);
        let opened = AtomicBool::new(false);

        let error = export_stream(stream_address(&mut stream), open_probe(&opened))
            .await
            .unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(
            error.message,
            "Out stream pointer already holds a live stream"
        );
        assert!(!opened.load(Ordering::Relaxed));
        let mut out_array = FFI_ArrowArray::empty();
        assert_eq!(next(&mut stream, &mut out_array).await.unwrap().status, 0);
        assert_eq!(imported(out_array), batch(vec![7]));
    }

    #[tokio::test]
    async fn a_failure_to_open_the_result_leaves_the_slot_unwritten() {
        let mut slot = FFI_ArrowArrayStream::empty();

        let error = export_stream(stream_address(&mut slot), async {
            Err(invalid_argument("result is gone"))
        })
        .await
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(error.message, "result is gone");
        assert!(slot.release.is_none());
    }

    #[tokio::test]
    async fn an_opened_result_is_exported_into_the_slot() {
        let mut slot = FFI_ArrowArrayStream::empty();
        let opened = AtomicBool::new(false);

        export_stream(stream_address(&mut slot), open_probe(&opened))
            .await
            .unwrap();

        assert!(opened.load(Ordering::Relaxed));
        let mut out_array = FFI_ArrowArray::empty();
        assert_eq!(next(&mut slot, &mut out_array).await.unwrap().status, 0);
        assert_eq!(imported(out_array), batch(vec![8]));
    }

    fn application_error<T: std::fmt::Debug>(
        result: Result<T, ProtoError<DriverException>>,
    ) -> DriverException {
        match result {
            Err(ProtoError::Application(error)) => error,
            other => panic!("expected an application error, got {other:?}"),
        }
    }

    fn registered_result_set(client: &DatabaseDriverClient) -> ResultSetHandle {
        client
            .transport()
            .database_driver()
            .register_arrow_batch_as_result_set(&batch(vec![1, 2, 3]), reqwest::Client::new())
            .unwrap()
            .handle
            .into()
    }

    async fn read_through_rpcs(
        client: &DatabaseDriverClient,
        result_set_handle: ResultSetHandle,
    ) -> Vec<RecordBatch> {
        let mut stream = export_through_rpc(client, result_set_handle).await;

        let mut batches = Vec::new();
        for _ in 0..MAX_READS {
            let mut out_array = FFI_ArrowArray::empty();
            let response = read_next_through_rpc(client, &mut stream, &mut out_array).await;
            assert_eq!(response.status, 0);
            if out_array.is_released() {
                return batches;
            }
            batches.push(imported(out_array));
        }
        panic!("the stream did not end within {MAX_READS} reads");
    }

    async fn export_through_rpc(
        client: &DatabaseDriverClient,
        result_set_handle: ResultSetHandle,
    ) -> FFI_ArrowArrayStream {
        let mut stream = FFI_ArrowArrayStream::empty();
        client
            .result_set_export_stream(ResultSetExportStreamRequest {
                result_set_handle: Some(result_set_handle),
                out_stream: stream_address(&mut stream),
            })
            .await
            .unwrap();
        stream
    }

    async fn read_next_through_rpc(
        client: &DatabaseDriverClient,
        stream: &mut FFI_ArrowArrayStream,
        out_array: &mut FFI_ArrowArray,
    ) -> ArrowStreamGetNextBatchResponse {
        client
            .arrow_stream_get_next_batch(ArrowStreamGetNextBatchRequest {
                stream: stream_address(stream),
                out_array: out_array_address(out_array),
            })
            .await
            .unwrap()
    }

    fn ipc_base64(schema: &Schema, batches: &[RecordBatch]) -> String {
        let mut buf = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buf, schema).unwrap();
            for batch in batches {
                writer.write(batch).unwrap();
            }
            writer.finish().unwrap();
        }
        BASE64.encode(&buf)
    }

    fn register_rowset(
        client: &DatabaseDriverClient,
        data: RowsetData,
        columns: Vec<ColumnMetadata>,
    ) -> ResultSetHandle {
        client
            .transport()
            .database_driver()
            .register_rowset(data, columns, reqwest::Client::new())
            .handle
            .into()
    }

    fn prefetched(batches: &[RecordBatch]) -> RowsetData {
        RowsetData::ArrowMultiChunk {
            initial_base64_opt: Some(ipc_base64(&schema(), batches)),
            chunk_download_data: Vec::new(),
        }
    }

    fn column(name: &str, column_type: &str, nullable: bool) -> ColumnMetadata {
        ColumnMetadata {
            name: name.to_string(),
            r#type: column_type.to_string(),
            precision: None,
            scale: None,
            length: None,
            byte_length: None,
            nullable,
            dimension: None,
            fixed: false,
            column_src_database: String::new(),
            column_src_schema: String::new(),
            column_src_table: String::new(),
            is_auto_increment: false,
            ext_col_type_name: String::new(),
            udt_output_type: String::new(),
            fields: Vec::new(),
        }
    }

    fn exported_schema(stream: &mut FFI_ArrowArrayStream) -> Schema {
        let mut ffi_schema = FFI_ArrowSchema::empty();
        let get_schema = stream.get_schema.unwrap();
        // SAFETY: `stream` is a live stream the core exported and `ffi_schema` is released
        // `ArrowSchema` storage that `get_schema` fills.
        assert_eq!(unsafe { get_schema(stream, &mut ffi_schema) }, 0);
        Schema::try_from(&ffi_schema).unwrap()
    }

    fn last_error(stream: &mut FFI_ArrowArrayStream) -> String {
        let get_last_error = stream.get_last_error.unwrap();
        // SAFETY: `stream` is a live stream the core exported whose last operation failed.
        let text = unsafe { get_last_error(stream) };
        assert!(!text.is_null(), "get_last_error returned null");
        // SAFETY: a non-null `get_last_error` result is a NUL-terminated string that stays valid
        // until the next operation on the stream.
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn an_exported_result_set_is_read_through_the_rpcs_until_the_end() {
        let client = database_driver_client();

        let batches = read_through_rpcs(&client, registered_result_set(&client)).await;

        assert_eq!(batches, vec![batch(vec![1, 2, 3])]);
    }

    #[tokio::test]
    async fn a_prefetched_result_set_is_read_through_the_rpcs_until_the_end() {
        let client = database_driver_client();
        let result_set_handle =
            register_rowset(&client, prefetched(&[batch(vec![4, 5])]), Vec::new());

        let batches = read_through_rpcs(&client, result_set_handle).await;

        assert_eq!(batches, vec![batch(vec![4, 5])]);
    }

    #[tokio::test]
    async fn exporting_without_a_result_set_handle_is_rejected_and_leaves_the_slot_unwritten() {
        let client = database_driver_client();
        let mut stream = FFI_ArrowArrayStream::empty();

        let error = application_error(
            client
                .result_set_export_stream(ResultSetExportStreamRequest {
                    result_set_handle: None,
                    out_stream: stream_address(&mut stream),
                })
                .await,
        );

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(error.message, "ResultSet handle is required");
        assert!(stream.release.is_none());
    }

    #[tokio::test]
    async fn exporting_an_unknown_result_set_fails_and_leaves_the_slot_unwritten() {
        let client = database_driver_client();
        let mut stream = FFI_ArrowArrayStream::empty();

        let error = application_error(
            client
                .result_set_export_stream(ResultSetExportStreamRequest {
                    result_set_handle: Some(ResultSetHandle {
                        id: 999_999,
                        magic: 0,
                    }),
                    out_stream: stream_address(&mut stream),
                })
                .await,
        );

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(
            error.message,
            "Invalid argument: result_handle: ResultSet handle not found"
        );
        assert!(stream.release.is_none());
    }

    #[tokio::test]
    async fn a_cancelled_export_reports_cancelled_and_leaves_the_slot_unwritten() {
        let client = database_driver_client();
        let result_set_handle = registered_result_set(&client);
        let (operation, _token) = client.transport().register();
        client.transport().cancel(operation);
        let mut stream = FFI_ArrowArrayStream::empty();

        let error = application_error(
            client
                .result_set_export_stream_cancellable(
                    operation,
                    ResultSetExportStreamRequest {
                        result_set_handle: Some(result_set_handle),
                        out_stream: stream_address(&mut stream),
                    },
                )
                .await,
        );

        assert_eq!(error.kind, ErrorKind::Cancelled as i32);
        assert_eq!(error.message, "Operation was cancelled");
        assert!(stream.release.is_none());
    }

    #[tokio::test]
    async fn a_read_cancelled_before_it_starts_reports_cancelled_without_calling_get_next() {
        let client = database_driver_client();
        let (reader, reader_calls) = counting_reader(vec![Ok(batch(vec![1]))]);
        let mut stream = core_stream(reader);
        let mut out_array = FFI_ArrowArray::empty();
        let (operation, _token) = client.transport().register();
        client.transport().cancel(operation);

        let error = application_error(
            client
                .arrow_stream_get_next_batch_cancellable(
                    operation,
                    ArrowStreamGetNextBatchRequest {
                        stream: stream_address(&mut stream),
                        out_array: out_array_address(&mut out_array),
                    },
                )
                .await,
        );

        assert_eq!(error.kind, ErrorKind::Cancelled as i32);
        assert_eq!(reader_calls.load(Ordering::Relaxed), 0);
        assert!(out_array.is_released());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelling_a_read_waits_for_get_next_and_returns_its_batch() {
        let (entered_tx, entered_rx) = sync_channel(1);
        let (proceed_tx, proceed_rx) = sync_channel::<()>(1);
        let get_next_returned = Arc::new(AtomicBool::new(false));
        let returned = Arc::clone(&get_next_returned);
        let mut pending = Some(batch(vec![42]));
        let mut stream = core_stream(ScriptedReader(move || {
            let batch = pending.take()?;
            let _ = entered_tx.send(());
            let _ = proceed_rx.recv_timeout(BOUND);
            returned.store(true, Ordering::Release);
            Some(Ok(batch))
        }));
        let mut out_array = FFI_ArrowArray::empty();
        let request = ArrowStreamGetNextBatchRequest {
            stream: stream_address(&mut stream),
            out_array: out_array_address(&mut out_array),
        };
        let client = Arc::new(database_driver_client());
        let (operation, token) = client.transport().register();
        let call = tokio::spawn({
            let client = Arc::clone(&client);
            async move {
                let response = client
                    .arrow_stream_get_next_batch_cancellable(operation, request)
                    .await;
                (response, get_next_returned.load(Ordering::Acquire))
            }
        });

        tokio::task::spawn_blocking(move || entered_rx.recv_timeout(BOUND))
            .await
            .unwrap()
            .expect("get_next was not called");
        client.transport().cancel(operation);
        assert!(token.is_cancelled());
        proceed_tx.send(()).unwrap();
        let (response, get_next_had_returned) = tokio::time::timeout(BOUND, call)
            .await
            .expect("the read did not complete after get_next returned")
            .unwrap();

        assert!(
            get_next_had_returned,
            "the read completed while get_next was still running: {response:?}"
        );
        assert_eq!(
            response.unwrap(),
            ArrowStreamGetNextBatchResponse {
                status: 0,
                error_message: None,
            }
        );
        assert_eq!(imported(out_array), batch(vec![42]));
    }

    #[tokio::test]
    async fn a_stream_the_core_did_not_export_is_rejected_before_get_next() {
        let state = StubState::default();
        let mut stream = stub_stream(&state);
        let mut out_array = FFI_ArrowArray::empty();

        let error = next(&mut stream, &mut out_array).await.unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(error.message, "Stream was not exported by the core");
        assert_eq!(state.get_next_calls.load(Ordering::Relaxed), 0);
        assert!(out_array.is_released());
    }

    #[tokio::test]
    async fn a_nul_byte_in_the_error_text_is_replaced_and_the_status_kept() {
        let mut stream = exported(vec![
            Err(ArrowError::ExternalError("bad\0value".into())),
            Err(ArrowError::IoError(
                "lost\0connection".to_string(),
                std::io::Error::other("reset"),
            )),
        ]);

        let mut external = FFI_ArrowArray::empty();
        let external_response = next(&mut stream, &mut external).await.unwrap();
        let mut io = FFI_ArrowArray::empty();
        let io_response = next(&mut stream, &mut io).await.unwrap();

        assert_eq!(
            external_response,
            ArrowStreamGetNextBatchResponse {
                status: EINVAL,
                error_message: Some("External error: bad\u{FFFD}value".to_string()),
            }
        );
        assert_eq!(
            io_response,
            ArrowStreamGetNextBatchResponse {
                status: EIO,
                error_message: Some("Io error: lost\u{FFFD}connection".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn a_panicking_reader_fails_that_read_and_every_later_one() {
        let mut calls = 0;
        let mut stream = core_stream(ScriptedReader(move || {
            calls += 1;
            if calls == 1 {
                panic!("reader exploded");
            }
            Some(Ok(batch(vec![1])))
        }));

        let mut first = FFI_ArrowArray::empty();
        let first_response = next(&mut stream, &mut first).await.unwrap();
        let mut second = FFI_ArrowArray::empty();
        let second_response = next(&mut stream, &mut second).await.unwrap();

        assert_eq!(
            first_response,
            ArrowStreamGetNextBatchResponse {
                status: EINVAL,
                error_message: Some(
                    "External error: Result set reader panicked: reader exploded".to_string()
                ),
            }
        );
        assert_eq!(
            second_response,
            ArrowStreamGetNextBatchResponse {
                status: EINVAL,
                error_message: Some(
                    "External error: Result set reader panicked while reading an earlier batch"
                        .to_string()
                ),
            }
        );
        assert!(first.is_released());
        assert!(second.is_released());
    }

    #[tokio::test]
    async fn reading_a_prefetched_stream_on_a_runtime_thread_fails_instead_of_aborting() {
        let client = database_driver_client();
        let result_set_handle = register_rowset(&client, prefetched(&[batch(vec![1])]), Vec::new());
        let mut stream = export_through_rpc(&client, result_set_handle).await;
        let mut out_array = FFI_ArrowArray::empty();
        let get_next = stream.get_next.unwrap();

        // SAFETY: `stream` is a live stream the core exported and `out_array` is released
        // `ArrowArray` storage that `get_next` fills.
        let status = unsafe { get_next(&mut stream, &mut out_array) };

        assert_eq!(status, EINVAL);
        assert!(out_array.is_released());
        let message = last_error(&mut stream);
        assert!(
            message.starts_with("External error: Result set reader panicked: Cannot block the current thread from within a runtime"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn the_exported_schema_carries_column_nullability_and_external_type_names() {
        let schema = Schema::new(vec![
            Field::new("N", DataType::Int64, false),
            Field::new("G", DataType::Utf8, true),
        ]);
        let rows = RecordBatch::try_new(
            Arc::new(schema.clone()),
            vec![
                Arc::new(Int64Array::from(vec![1])),
                Arc::new(StringArray::from(vec![Some("POINT(1 2)")])),
            ],
        )
        .unwrap();
        let client = database_driver_client();
        let result_set_handle = register_rowset(
            &client,
            RowsetData::ArrowSingleChunk {
                chunk_base64: ipc_base64(&schema, &[rows]),
            },
            vec![column("N", "FIXED", false), column("G", "GEOGRAPHY", true)],
        );

        let mut stream = export_through_rpc(&client, result_set_handle).await;

        assert_eq!(
            exported_schema(&mut stream),
            Schema::new(vec![
                Field::new("N", DataType::Int64, false).with_metadata(HashMap::from([(
                    "nullable".to_string(),
                    "false".to_string()
                )])),
                Field::new("G", DataType::Utf8, true).with_metadata(HashMap::from([
                    ("nullable".to_string(), "true".to_string()),
                    ("extTypeName".to_string(), "GEOGRAPHY".to_string()),
                ])),
            ])
        );
    }

    #[tokio::test]
    async fn an_empty_result_ends_on_the_first_read_and_still_exports_its_schema() {
        let client = database_driver_client();
        let result_set_handle = register_rowset(
            &client,
            RowsetData::ArrowSingleChunk {
                chunk_base64: ipc_base64(&schema(), &[]),
            },
            vec![column("n", "FIXED", false)],
        );
        let mut stream = export_through_rpc(&client, result_set_handle).await;

        let mut out_array = FFI_ArrowArray::empty();
        let response = read_next_through_rpc(&client, &mut stream, &mut out_array).await;

        assert_eq!(
            response,
            ArrowStreamGetNextBatchResponse {
                status: 0,
                error_message: None,
            }
        );
        assert!(out_array.is_released());
        assert_eq!(
            exported_schema(&mut stream),
            Schema::new(vec![Field::new("n", DataType::Int64, false).with_metadata(
                HashMap::from([("nullable".to_string(), "false".to_string())])
            )])
        );
    }

    #[tokio::test]
    async fn a_failed_chunk_download_is_reported_by_the_read_that_needs_it() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/chunk"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let client = database_driver_client();
        let result_set_handle = register_rowset(
            &client,
            RowsetData::ArrowMultiChunk {
                initial_base64_opt: Some(ipc_base64(&schema(), &[batch(vec![1])])),
                chunk_download_data: vec![ChunkDownloadData {
                    url: format!("{}/chunk", server.uri()),
                    row_count: 1,
                    uncompressed_size: 0,
                    compressed_size: 0,
                    headers: HashMap::new(),
                }],
            },
            Vec::new(),
        );
        let mut stream = export_through_rpc(&client, result_set_handle).await;

        let mut first = FFI_ArrowArray::empty();
        let first_response = read_next_through_rpc(&client, &mut stream, &mut first).await;
        let mut failed = FFI_ArrowArray::empty();
        let failed_response = read_next_through_rpc(&client, &mut stream, &mut failed).await;

        assert_eq!(first_response.status, 0);
        assert_eq!(imported(first), batch(vec![1]));
        assert_eq!(
            failed_response,
            ArrowStreamGetNextBatchResponse {
                status: EINVAL,
                error_message: Some(
                    "External error: Snowflake responded with non-successful HTTP status"
                        .to_string()
                ),
            }
        );
        assert!(failed.is_released());
    }

    #[tokio::test]
    async fn the_out_stream_is_validated_before_the_result_set_is_looked_up() {
        let client = database_driver_client();
        let unknown = ResultSetHandle {
            id: 999_999,
            magic: 0,
        };
        let mut live = exported(vec![Ok(batch(vec![7]))]);

        let missing = application_error(
            client
                .result_set_export_stream(ResultSetExportStreamRequest {
                    result_set_handle: Some(unknown),
                    out_stream: None,
                })
                .await,
        );
        let occupied = application_error(
            client
                .result_set_export_stream(ResultSetExportStreamRequest {
                    result_set_handle: Some(unknown),
                    out_stream: stream_address(&mut live),
                })
                .await,
        );

        assert_eq!(missing.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(missing.message, "Out stream pointer is required");
        assert_eq!(occupied.kind, ErrorKind::InvalidArgument as i32);
        assert_eq!(
            occupied.message,
            "Out stream pointer already holds a live stream"
        );
        let mut out_array = FFI_ArrowArray::empty();
        assert_eq!(next(&mut live, &mut out_array).await.unwrap().status, 0);
        assert_eq!(imported(out_array), batch(vec![7]));
    }

    #[tokio::test]
    async fn interleaved_reads_of_two_exported_streams_keep_their_own_cursors() {
        let client = database_driver_client();
        let first_handle = register_rowset(&client, prefetched(&[batch(vec![1])]), Vec::new());
        let second_handle = register_rowset(&client, prefetched(&[batch(vec![2])]), Vec::new());
        let mut streams = [
            export_through_rpc(&client, first_handle).await,
            export_through_rpc(&client, second_handle).await,
            export_through_rpc(&client, first_handle).await,
        ];

        let mut reads = Vec::new();
        for index in [0, 1, 2, 0, 1, 2] {
            let mut out_array = FFI_ArrowArray::empty();
            assert_eq!(
                read_next_through_rpc(&client, &mut streams[index], &mut out_array)
                    .await
                    .status,
                0
            );
            reads.push((!out_array.is_released()).then(|| imported(out_array)));
        }

        assert_eq!(
            reads,
            vec![
                Some(batch(vec![1])),
                Some(batch(vec![2])),
                Some(batch(vec![1])),
                None,
                None,
                None,
            ]
        );
    }
}
