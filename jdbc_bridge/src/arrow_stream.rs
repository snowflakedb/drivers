use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

use arrow::array::{Array, RecordBatch, RecordBatchReader};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use sf_core::utils::sync::MutexRecoverExt;
use snafu::{Location, ResultExt, Snafu};

const BOOL: u8 = 1;
const I8: u8 = 2;
const I16: u8 = 3;
const I32: u8 = 4;
const I64: u8 = 5;
const F32: u8 = 6;
const F64: u8 = 7;
const UTF8: u8 = 8;
const BINARY: u8 = 9;
const DATE32: u8 = 10;
const DECIMAL128: u8 = 11;
const STRUCT: u8 = 12;
const FIXED_SIZE_LIST: u8 = 13;

pub(crate) const BUFFER_VALIDITY: u8 = 1;
pub(crate) const BUFFER_OFFSETS: u8 = 2;
pub(crate) const BUFFER_VALUES: u8 = 3;

struct Registry {
    next_handle: u64,
    streams: HashMap<u64, Arc<Stream>>,
}

pub(crate) struct ArrowStreamRegistry {
    inner: Mutex<Registry>,
}

/// One open stream. `closing` lives outside `state` so `close` can detach the stream
/// without waiting for a `reader.next()` that is already in flight.
struct Stream {
    /// Set by `close` before it tries for `state`. An export that is already inside
    /// `reader.next()` re-reads this when the read returns and discards the batch
    /// instead of publishing it.
    closing: AtomicBool,
    state: Mutex<StreamState>,
}

struct StreamState {
    reader: ArrowArrayStreamReader,
    phase: Phase,
}

enum Phase {
    /// `replay` is a batch already pulled from the reader whose export callback failed.
    /// The next export returns it again instead of reading another batch.
    /// `published` is the last batch a successful export returned. It keeps those
    /// buffers alive until the next successful export or `close`.
    Open {
        replay: Option<BatchRef>,
        published: Option<BatchRef>,
    },
    /// A describe or reader failure. Later exports return `error`.
    /// `published` keeps the last successful export alive until `close`.
    Failed {
        error: ArrowStreamError,
        published: Option<BatchRef>,
    },
    /// Set by whichever of `close` or an in-flight export first takes the stream
    /// mutex after `close` detaches the stream, before this state is dropped. It
    /// drops any retained batch at that point rather than at the last `Arc` drop.
    Closed,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportSource(String);

impl From<String> for ExportSource {
    fn from(detail: String) -> Self {
        Self(detail)
    }
}

impl Display for ExportSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExportSource {}

#[derive(Debug, Clone, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ArrowStreamError {
    #[snafu(display("Arrow stream error: {source}"))]
    Arrow {
        #[snafu(source(from(ArrowError, Arc::new)))]
        source: Arc<ArrowError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Arrow stream pointer is null or misaligned"))]
    InvalidPointer {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Arrow stream handle {handle} is not open"))]
    MissingHandle {
        handle: u64,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Arrow column {column:?} is unsupported: {detail}"))]
    UnsupportedType {
        column: String,
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Arrow column \"batch\" is unsupported: row count {row_count} exceeds i32"))]
    BatchRowCount {
        row_count: usize,
        source: std::num::TryFromIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Arrow column \"schema\" is unsupported: encoded length {len} exceeds i32"))]
    EncodedLength {
        len: usize,
        source: std::num::TryFromIntError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{source}"))]
    Export {
        #[snafu(source(from(String, ExportSource::from)))]
        source: ExportSource,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Prefix stamped onto every Java-visible message carrying
/// [`ArrowStreamError::UnsupportedType`], [`ArrowStreamError::BatchRowCount`], or
/// [`ArrowStreamError::EncodedLength`]. `NativeArrowStream.arrowFailure` matches it to pick
/// `ErrorCode.FEATURE_UNSUPPORTED`. Change in lockstep with the Java constant of the same name.
pub(crate) const UNSUPPORTED_COLUMN_TAG: &str = "UNSUPPORTED_COLUMN:";

impl ArrowStreamError {
    pub(crate) fn java_message(&self) -> String {
        match self {
            Self::UnsupportedType { .. }
            | Self::BatchRowCount { .. }
            | Self::EncodedLength { .. } => {
                format!("{UNSUPPORTED_COLUMN_TAG}{self}")
            }
            _ => self.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BufferRef {
    pub role: u8,
    pub address: u64,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ArrayRef {
    pub buffers: Vec<BufferRef>,
    pub children: Vec<ArrayRef>,
}

#[derive(Debug, Clone)]
pub(crate) struct BatchRef {
    pub row_count: i32,
    pub columns: Vec<ArrayRef>,
    /// Owns the buffers named by `columns`.
    retained: RecordBatch,
}

impl ArrowStreamRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Registry {
                next_handle: 1,
                streams: HashMap::new(),
            }),
        }
    }

    /// `pointer` is `0` or a `Box::into_raw` `FFI_ArrowArrayStream`. The caller does
    /// not free `pointer` after this returns. This consumes that allocation, including
    /// when `ArrowArrayStreamReader::try_new` fails. A null or misaligned pointer is
    /// not consumed.
    pub(crate) fn open(&self, pointer: i64) -> Result<u64, ArrowStreamError> {
        let raw = pointer as usize as *mut FFI_ArrowArrayStream;
        if raw.is_null() || !raw.is_aligned() {
            return Err(InvalidPointerSnafu.build());
        }
        // SAFETY: `raw` is non-null, aligned, and a `Box::into_raw` of an
        // `FFI_ArrowArrayStream`. `from_raw` moves that value out and leaves an
        // empty stream in the allocation. The caller does not free `raw`.
        let stream = unsafe { FFI_ArrowArrayStream::from_raw(raw) };
        let reader = ArrowArrayStreamReader::try_new(stream);
        // SAFETY: `from_raw` left an empty stream in this allocation (`release` is
        // `None`), so dropping the box frees it and does not run a release callback.
        unsafe { drop(Box::from_raw(raw)) };
        let reader = reader.context(ArrowSnafu)?;
        let stream = Arc::new(Stream {
            closing: AtomicBool::new(false),
            state: Mutex::new(StreamState {
                reader,
                phase: Phase::Open {
                    replay: None,
                    published: None,
                },
            }),
        });
        let mut registry = self.inner.lock_recover();
        let handle = registry.next_handle;
        registry.next_handle = registry.next_handle.wrapping_add(1);
        registry.streams.insert(handle, stream);
        Ok(handle)
    }

    fn stream(&self, handle: u64) -> Result<Arc<Stream>, ArrowStreamError> {
        self.inner
            .lock_recover()
            .streams
            .get(&handle)
            .cloned()
            .ok_or_else(|| MissingHandleSnafu { handle }.build())
    }

    pub(crate) fn schema(&self, handle: u64) -> Result<Vec<u8>, ArrowStreamError> {
        let stream = self.stream(handle)?;
        if stream.closing.load(Ordering::SeqCst) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        let mut state = stream.state.lock_recover();
        if matches!(state.phase, Phase::Closed) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        let encoded = encode_schema(state.reader.schema().as_ref());
        if self.detached(handle, &stream, &mut state, &mut None) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        encoded
    }

    /// `export` runs while this stream's mutex is held. A callback that calls back
    /// into this registry on the same handle deadlocks. This stream keeps the
    /// returned batch until the next successful export or `close`. A [`BatchRef`]
    /// cloned from the callback keeps its own buffers alive after both.
    ///
    /// A concurrent [`Self::close`] does not wait for this call. One observed by
    /// either `closing` check — before the read or after the callback — makes this
    /// return [`ArrowStreamError::MissingHandle`], discard the batch, and remove the
    /// registry entry that `close` left behind.
    ///
    /// A `close` that lands after the second load took the
    /// [`TryLockError::WouldBlock`] path while this call held the stream mutex, so it
    /// left the registry entry in place. That entry, not this call's `Arc`, is the
    /// batch's last owner, so the batch this call publishes stays alive for the
    /// caller about to read it. Nothing reclaims the stream afterwards: a `close`
    /// racing a fetch that way leaks one reader and one batch rather than freeing
    /// memory the caller holds addresses into.
    pub(crate) fn export_next_batch<T>(
        &self,
        handle: u64,
        export: impl FnOnce(Option<&BatchRef>) -> Result<T, String>,
    ) -> Result<T, ArrowStreamError> {
        let stream = self.stream(handle)?;
        if stream.closing.load(Ordering::SeqCst) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        let mut state = stream.state.lock_recover();
        if matches!(state.phase, Phase::Closed) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        if let Phase::Failed { error, .. } = &state.phase {
            return Err(error.clone());
        }
        let replay = match &mut state.phase {
            Phase::Open { replay, .. } => replay.take(),
            Phase::Failed { .. } | Phase::Closed => None,
        };
        let described = match replay {
            Some(pending) => Some(pending),
            None => match read_next_batch(&mut state) {
                Ok(batch) => batch,
                Err(error) => {
                    let published = match &mut state.phase {
                        Phase::Open { published, .. } => published.take(),
                        Phase::Failed { published, .. } => published.take(),
                        Phase::Closed => None,
                    };
                    state.phase = Phase::Failed {
                        error: error.clone(),
                        published,
                    };
                    return Err(error);
                }
            },
        };
        let mut described = described;
        if self.detached(handle, &stream, &mut state, &mut described) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        let exported = export(described.as_ref());
        // Re-checked: `close` may have landed while the callback was running, in which
        // case nothing retains these buffers once this call returns, so the export has
        // to fail rather than report a batch the caller may still read.
        if self.detached(handle, &stream, &mut state, &mut described) {
            return Err(MissingHandleSnafu { handle }.build());
        }
        if let Phase::Open { replay, published } = &mut state.phase {
            if exported.is_err() {
                *replay = described;
            } else {
                *published = described;
            }
        }
        exported.context(ExportSnafu)
    }

    /// Detaches `handle` and returns without waiting for an in-flight export. The
    /// exporting thread observes `closing` and discards its batch.
    ///
    /// Whichever of the two holds the stream mutex owns the registry entry: this call
    /// removes it when `try_lock` succeeds, and the lock holder removes it in
    /// [`Self::detached`] when it does not. Leaving the entry in place on the
    /// [`TryLockError::WouldBlock`] path is what keeps the batch an in-flight export
    /// is about to publish alive — that entry outlives the export, where the
    /// exporting thread's `Arc` does not.
    pub(crate) fn close(&self, handle: u64) {
        let Ok(stream) = self.stream(handle) else {
            return;
        };
        stream.closing.store(true, Ordering::SeqCst);
        match stream.state.try_lock() {
            Ok(mut state) => {
                state.phase = Phase::Closed;
                drop(state);
                self.remove(handle);
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                tracing::error!(
                    handle,
                    "recovered a poisoned Arrow stream mutex while closing (a thread \
                     previously panicked while holding it)",
                );
                poisoned.into_inner().phase = Phase::Closed;
                self.remove(handle);
            }
            // An export or a `schema` call holds the stream and owns the removal.
            Err(TryLockError::WouldBlock) => {}
        }
    }

    /// `true` when `close` detached this stream while the caller held its mutex. The
    /// phase moves to [`Phase::Closed`], `described` is dropped, and the registry
    /// entry `close` left to the lock holder is removed, so the caller never hands
    /// out addresses nothing keeps alive.
    fn detached(
        &self,
        handle: u64,
        stream: &Stream,
        state: &mut StreamState,
        described: &mut Option<BatchRef>,
    ) -> bool {
        if !stream.closing.load(Ordering::SeqCst) {
            return false;
        }
        state.phase = Phase::Closed;
        *described = None;
        self.remove(handle);
        true
    }

    fn remove(&self, handle: u64) {
        self.inner.lock_recover().streams.remove(&handle);
    }

    #[cfg(test)]
    fn open_handles(&self) -> usize {
        self.inner.lock_recover().streams.len()
    }
}

fn read_next_batch(state: &mut StreamState) -> Result<Option<BatchRef>, ArrowStreamError> {
    match state.reader.next() {
        Some(Ok(batch)) => describe_batch(batch).map(Some),
        Some(Err(error)) => Err(error).context(ArrowSnafu),
        None => Ok(None),
    }
}

fn encode_schema(schema: &Schema) -> Result<Vec<u8>, ArrowStreamError> {
    let mut output = Vec::new();
    write_len(&mut output, schema.fields().len())?;
    for field in schema.fields() {
        encode_field_schema(&mut output, field)?;
    }
    Ok(output)
}

fn encode_field_schema(output: &mut Vec<u8>, field: &Field) -> Result<(), ArrowStreamError> {
    write_string(output, field.name())?;
    write_metadata(output, field)?;
    output.push(u8::from(field.is_nullable()));
    match kind_of(field.name(), field.data_type())? {
        Kind::Bool => output.push(BOOL),
        Kind::Fixed { tag, .. } | Kind::Variable { tag } => output.push(tag),
        Kind::Decimal { precision, scale } => {
            output.push(DECIMAL128);
            write_i32(output, precision);
            write_i32(output, scale);
        }
        Kind::Struct => {
            output.push(STRUCT);
            let DataType::Struct(children) = field.data_type() else {
                return Err(unsupported(
                    field.name(),
                    "struct type is missing its fields".to_string(),
                ));
            };
            write_len(output, children.len())?;
            for child in children {
                encode_field_schema(output, child)?;
            }
        }
        Kind::FixedList { list_size } => {
            output.push(FIXED_SIZE_LIST);
            write_i32(output, list_size);
            let DataType::FixedSizeList(child, _) = field.data_type() else {
                return Err(unsupported(
                    field.name(),
                    "fixed-size list is missing its child".to_string(),
                ));
            };
            encode_field_schema(output, child.as_ref())?;
        }
    }
    Ok(())
}

enum Kind {
    Bool,
    Fixed { tag: u8, width: usize },
    Variable { tag: u8 },
    Decimal { precision: i32, scale: i32 },
    Struct,
    FixedList { list_size: i32 },
}

fn kind_of(name: &str, data_type: &DataType) -> Result<Kind, ArrowStreamError> {
    match data_type {
        DataType::Boolean => Ok(Kind::Bool),
        DataType::Int8 => Ok(Kind::Fixed { tag: I8, width: 1 }),
        DataType::Int16 => Ok(Kind::Fixed { tag: I16, width: 2 }),
        DataType::Int32 => Ok(Kind::Fixed { tag: I32, width: 4 }),
        DataType::Date32 => Ok(Kind::Fixed {
            tag: DATE32,
            width: 4,
        }),
        DataType::Float32 => Ok(Kind::Fixed { tag: F32, width: 4 }),
        DataType::Int64 => Ok(Kind::Fixed { tag: I64, width: 8 }),
        DataType::Float64 => Ok(Kind::Fixed { tag: F64, width: 8 }),
        DataType::Utf8 => Ok(Kind::Variable { tag: UTF8 }),
        DataType::Binary => Ok(Kind::Variable { tag: BINARY }),
        DataType::Decimal128(precision, scale) => Ok(Kind::Decimal {
            precision: i32::from(*precision),
            scale: i32::from(*scale),
        }),
        DataType::Struct(_) => Ok(Kind::Struct),
        DataType::FixedSizeList(_, list_size) => Ok(Kind::FixedList {
            list_size: *list_size,
        }),
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => Err(
            unsupported(name, format!("unsigned integer type {data_type}")),
        ),
        DataType::LargeUtf8 => Err(unsupported(name, "large utf8".to_string())),
        DataType::LargeBinary => Err(unsupported(name, "large binary".to_string())),
        DataType::Dictionary(_, _) => {
            Err(unsupported(name, "dictionary-encoded columns".to_string()))
        }
        other => Err(unsupported(name, other.to_string())),
    }
}

fn describe_batch(batch: RecordBatch) -> Result<BatchRef, ArrowStreamError> {
    let rows = i32::try_from(batch.num_rows()).context(BatchRowCountSnafu {
        row_count: batch.num_rows(),
    })?;
    let mut columns = Vec::with_capacity(batch.num_columns());
    for (field, column) in batch.schema().fields().iter().zip(batch.columns()) {
        columns.push(describe_array(field, column.as_ref())?);
    }
    Ok(BatchRef {
        row_count: rows,
        columns,
        retained: batch,
    })
}

fn describe_array(field: &Field, array: &dyn Array) -> Result<ArrayRef, ArrowStreamError> {
    let name = field.name();
    match kind_of(name, array.data_type())? {
        Kind::Bool => describe_bool(name, array),
        Kind::Fixed { width, .. } => describe_fixed(name, array, width),
        Kind::Decimal { .. } => describe_fixed(name, array, 16),
        Kind::Variable { .. } => describe_variable(name, array),
        Kind::Struct => describe_struct(field, array),
        Kind::FixedList { .. } => describe_fixed_list(field, array),
    }
}

fn describe_bool(name: &str, array: &dyn Array) -> Result<ArrayRef, ArrowStreamError> {
    let data = array.to_data();
    let mut buffers = Vec::with_capacity(2);
    if let Some(validity) = describe_validity(name, &data)? {
        buffers.push(validity);
    }
    let values = data
        .buffers()
        .first()
        .ok_or_else(|| unsupported(name, "boolean array missing values buffer".to_string()))?;
    buffers.push(bit_slice(
        name,
        values,
        data.offset(),
        data.len(),
        BUFFER_VALUES,
    )?);
    Ok(ArrayRef {
        buffers,
        children: Vec::new(),
    })
}

fn describe_fixed(
    name: &str,
    array: &dyn Array,
    width: usize,
) -> Result<ArrayRef, ArrowStreamError> {
    let data = array.to_data();
    let mut buffers = Vec::with_capacity(2);
    if let Some(validity) = describe_validity(name, &data)? {
        buffers.push(validity);
    }
    let values = data
        .buffers()
        .first()
        .ok_or_else(|| unsupported(name, "fixed-width array missing values buffer".to_string()))?;
    let byte_offset = data
        .offset()
        .checked_mul(width)
        .ok_or_else(|| unsupported(name, "value offset overflows".to_string()))?;
    let byte_len = data
        .len()
        .checked_mul(width)
        .ok_or_else(|| unsupported(name, "value length overflows".to_string()))?;
    buffers.push(byte_slice(
        name,
        values,
        byte_offset,
        byte_len,
        BUFFER_VALUES,
    )?);
    Ok(ArrayRef {
        buffers,
        children: Vec::new(),
    })
}

fn describe_variable(name: &str, array: &dyn Array) -> Result<ArrayRef, ArrowStreamError> {
    let data = array.to_data();
    let mut buffers = Vec::with_capacity(3);
    if let Some(validity) = describe_validity(name, &data)? {
        buffers.push(validity);
    }
    let offsets = data
        .buffers()
        .first()
        .ok_or_else(|| unsupported(name, "variable array missing offsets buffer".to_string()))?;
    let values = data
        .buffers()
        .get(1)
        .ok_or_else(|| unsupported(name, "variable array missing values buffer".to_string()))?;
    let offset_bytes = data
        .offset()
        .checked_mul(4)
        .ok_or_else(|| unsupported(name, "offset index overflows".to_string()))?;
    let offset_len = data
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| unsupported(name, "offset length overflows".to_string()))?;
    let last_index = data
        .offset()
        .checked_add(data.len())
        .ok_or_else(|| unsupported(name, "offset index overflows".to_string()))?;
    let last = read_i32_at(name, offsets, last_index)?;
    if last < 0 || last as usize > values.len() {
        return Err(unsupported(
            name,
            format!(
                "value offsets end at {last}, past the values buffer of length {}",
                values.len()
            ),
        ));
    }
    buffers.push(byte_slice(
        name,
        offsets,
        offset_bytes,
        offset_len,
        BUFFER_OFFSETS,
    )?);
    buffers.push(byte_slice(name, values, 0, values.len(), BUFFER_VALUES)?);
    Ok(ArrayRef {
        buffers,
        children: Vec::new(),
    })
}

fn describe_struct(field: &Field, array: &dyn Array) -> Result<ArrayRef, ArrowStreamError> {
    let name = field.name();
    let data = array.to_data();
    let mut buffers = Vec::new();
    if let Some(validity) = describe_validity(name, &data)? {
        buffers.push(validity);
    }
    let DataType::Struct(child_fields) = field.data_type() else {
        return Err(unsupported(
            name,
            "struct array has no struct field".to_string(),
        ));
    };
    if child_fields.len() != data.child_data().len() {
        return Err(unsupported(
            name,
            format!(
                "struct has {} fields and {} child arrays",
                child_fields.len(),
                data.child_data().len()
            ),
        ));
    }
    let mut children = Vec::with_capacity(child_fields.len());
    for (child_field, child) in child_fields.iter().zip(data.child_data()) {
        // `StructArray` holds no logical offset of its own: both `Array::slice` and
        // `From<ArrayData>` narrow the children instead, so `data.offset()` is 0 here
        // and this is a no-op. It is applied anyway so a child that does arrive
        // unnarrowed still describes the rows the validity bits above cover, and so an
        // inconsistent length is reported rather than panicking in `ArrayData::slice`.
        let sliced = slice_child(name, child, data.offset(), data.len())?;
        let child_array = arrow::array::make_array(sliced);
        children.push(describe_array(child_field.as_ref(), child_array.as_ref())?);
    }
    Ok(ArrayRef { buffers, children })
}

/// Narrow `child` to the `len` elements at `offset`, rejecting a range past its end.
/// [`arrow::array::ArrayData::slice`] panics there, and a panic must not cross the
/// JNI boundary.
fn slice_child(
    name: &str,
    child: &arrow::array::ArrayData,
    offset: usize,
    len: usize,
) -> Result<arrow::array::ArrayData, ArrowStreamError> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| unsupported(name, "child slice overflows".to_string()))?;
    if end > child.len() {
        return Err(unsupported(
            name,
            format!(
                "child slice [{offset}, {end}) exceeds child length {}",
                child.len()
            ),
        ));
    }
    Ok(child.slice(offset, len))
}

fn describe_fixed_list(field: &Field, array: &dyn Array) -> Result<ArrayRef, ArrowStreamError> {
    let name = field.name();
    let data = array.to_data();
    let mut buffers = Vec::new();
    if let Some(validity) = describe_validity(name, &data)? {
        buffers.push(validity);
    }
    let DataType::FixedSizeList(child_field, list_size) = field.data_type() else {
        return Err(unsupported(
            name,
            "fixed-size list array has no list field".to_string(),
        ));
    };
    snafu::ensure!(
        *list_size >= 0,
        UnsupportedTypeSnafu {
            column: name.to_string(),
            detail: format!("negative list size {list_size}"),
        }
    );
    let list_size = *list_size as usize;
    let child = data
        .child_data()
        .first()
        .ok_or_else(|| unsupported(name, "fixed-size list missing child".to_string()))?;
    // Same no-op-by-construction translation as `describe_struct`, scaled by the list
    // width: element `i` occupies child positions `[i * list_size, (i+1) * list_size)`.
    let child_offset = data
        .offset()
        .checked_mul(list_size)
        .ok_or_else(|| unsupported(name, "child offset overflows".to_string()))?;
    let child_len = data
        .len()
        .checked_mul(list_size)
        .ok_or_else(|| unsupported(name, "child length overflows".to_string()))?;
    let sliced = slice_child(name, child, child_offset, child_len)?;
    let child_array = arrow::array::make_array(sliced);
    Ok(ArrayRef {
        buffers,
        children: vec![describe_array(child_field.as_ref(), child_array.as_ref())?],
    })
}

fn describe_validity(
    name: &str,
    data: &arrow::array::ArrayData,
) -> Result<Option<BufferRef>, ArrowStreamError> {
    let Some(nulls) = data.nulls() else {
        return Ok(None);
    };
    if nulls.null_count() == 0 {
        return Ok(None);
    }
    Ok(Some(bit_slice(
        name,
        nulls.buffer(),
        nulls.offset(),
        nulls.len(),
        BUFFER_VALIDITY,
    )?))
}

fn bit_slice(
    name: &str,
    buffer: &arrow::buffer::Buffer,
    bit_offset: usize,
    bit_len: usize,
    role: u8,
) -> Result<BufferRef, ArrowStreamError> {
    if !bit_offset.is_multiple_of(8) {
        // TODO(SNOW-4248904): a slice whose bit offset is not a multiple of 8 is
        // rejected. Copying those bits into an aligned buffer would accept it, and
        // that copy is deferred so byte-aligned batches stay pointer exports.
        return Err(unsupported(
            name,
            format!("bit offset {bit_offset} is not byte-aligned"),
        ));
    }
    let byte_offset = bit_offset / 8;
    let byte_len = bit_len.div_ceil(8);
    byte_slice(name, buffer, byte_offset, byte_len, role)
}

fn byte_slice(
    name: &str,
    buffer: &arrow::buffer::Buffer,
    byte_offset: usize,
    byte_len: usize,
    role: u8,
) -> Result<BufferRef, ArrowStreamError> {
    if byte_len == 0 {
        return Ok(BufferRef {
            role,
            address: 0,
            length: 0,
        });
    }
    if i32::try_from(byte_len).is_err() {
        return Err(unsupported(
            name,
            format!("buffer length {byte_len} exceeds i32"),
        ));
    }
    let end = byte_offset
        .checked_add(byte_len)
        .ok_or_else(|| unsupported(name, "buffer slice overflows".to_string()))?;
    if end > buffer.len() {
        return Err(unsupported(
            name,
            format!(
                "buffer slice [{byte_offset}, {end}) exceeds buffer length {}",
                buffer.len()
            ),
        ));
    }
    Ok(BufferRef {
        role,
        address: buffer.as_ptr() as u64 + byte_offset as u64,
        length: byte_len,
    })
}

fn read_i32_at(
    name: &str,
    buffer: &arrow::buffer::Buffer,
    index: usize,
) -> Result<i32, ArrowStreamError> {
    let start = index
        .checked_mul(4)
        .ok_or_else(|| unsupported(name, "offset index overflows".to_string()))?;
    let end = start
        .checked_add(4)
        .ok_or_else(|| unsupported(name, "offset index overflows".to_string()))?;
    let bytes = buffer
        .as_slice()
        .get(start..end)
        .ok_or_else(|| unsupported(name, "offsets buffer is short".to_string()))?;
    let mut encoded = [0u8; 4];
    encoded.copy_from_slice(bytes);
    Ok(i32::from_ne_bytes(encoded))
}

fn unsupported(name: &str, detail: String) -> ArrowStreamError {
    UnsupportedTypeSnafu {
        column: name.to_string(),
        detail,
    }
    .build()
}

fn write_metadata(output: &mut Vec<u8>, field: &Field) -> Result<(), ArrowStreamError> {
    let metadata = field.metadata();
    write_len(output, metadata.len())?;
    for (key, value) in metadata {
        write_string(output, key)?;
        write_string(output, value)?;
    }
    Ok(())
}

fn write_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ArrowStreamError> {
    write_len(output, bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), ArrowStreamError> {
    write_bytes(output, value.as_bytes())
}

fn write_len(output: &mut Vec<u8>, len: usize) -> Result<(), ArrowStreamError> {
    let len = i32::try_from(len).context(EncodedLengthSnafu { len })?;
    write_i32(output, len);
    Ok(())
}

fn write_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use arrow::array::{
        Array, BooleanArray, FixedSizeListArray, Int64Array, RecordBatch, RecordBatchIterator,
        StringArray, StructArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
    };
    use arrow::buffer::BooleanBuffer;
    use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
    use arrow::error::ArrowError;
    use arrow::record_batch::RecordBatchReader;

    use super::*;

    fn field(name: &str, logical_type: &str, data_type: DataType) -> Field {
        Field::new(name, data_type, true).with_metadata(HashMap::from([(
            "logicalType".to_string(),
            logical_type.to_string(),
        )]))
    }

    fn read_i32(input: &[u8], offset: &mut usize) -> i32 {
        let value = i32::from_le_bytes(input[*offset..*offset + 4].try_into().expect("i32"));
        *offset += 4;
        value
    }

    fn read_string(input: &[u8], offset: &mut usize) -> String {
        let length = read_i32(input, offset);
        let bytes = &input[*offset..*offset + length as usize];
        *offset += length as usize;
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    #[test]
    fn schema_encodes_metadata_without_values() {
        let fixed = field("N", "FIXED", DataType::Int64).with_metadata(HashMap::from([
            ("logicalType".to_string(), "FIXED".to_string()),
            ("scale".to_string(), "2".to_string()),
        ]));
        let schema = Schema::new(vec![
            field("B", "BOOLEAN", DataType::Boolean),
            fixed,
            field("S", "TEXT", DataType::Utf8),
        ]);
        let encoded = encode_schema(&schema).expect("schema encodes");
        let mut offset = 0;
        assert_eq!(read_i32(&encoded, &mut offset), 3);
        assert_eq!(read_string(&encoded, &mut offset), "B");
        assert_eq!(read_i32(&encoded, &mut offset), 1);
        assert_eq!(read_string(&encoded, &mut offset), "logicalType");
        assert_eq!(read_string(&encoded, &mut offset), "BOOLEAN");
        assert_eq!(encoded[offset], 1);
        assert_eq!(encoded[offset + 1], BOOL);
        offset += 2;
        assert_eq!(read_string(&encoded, &mut offset), "N");
        assert_eq!(read_i32(&encoded, &mut offset), 2);
        let mut metadata = HashMap::new();
        for _ in 0..2 {
            metadata.insert(
                read_string(&encoded, &mut offset),
                read_string(&encoded, &mut offset),
            );
        }
        assert_eq!(metadata.get("scale").map(String::as_str), Some("2"));
        assert_eq!(encoded[offset], 1);
        assert_eq!(encoded[offset + 1], I64);
        offset += 2;
        assert_eq!(read_string(&encoded, &mut offset), "S");
        assert_eq!(read_i32(&encoded, &mut offset), 1);
        assert_eq!(read_string(&encoded, &mut offset), "logicalType");
        assert_eq!(read_string(&encoded, &mut offset), "TEXT");
        assert_eq!(encoded[offset], 1);
        assert_eq!(encoded[offset + 1], UTF8);
        offset += 2;
        assert_eq!(offset, encoded.len());
    }

    #[test]
    fn batch_exports_buffer_addresses_without_copying_values() {
        let schema = Arc::new(Schema::new(vec![
            field("B", "BOOLEAN", DataType::Boolean),
            field("N", "FIXED", DataType::Int64),
            field("S", "TEXT", DataType::Utf8),
        ]));
        let bools = BooleanArray::from(vec![Some(true), None]);
        let numbers = Int64Array::from(vec![Some(123), Some(-5)]);
        let strings = StringArray::from(vec![Some("hello"), Some("world")]);
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(bools.clone()),
                Arc::new(numbers.clone()),
                Arc::new(strings.clone()),
            ],
        )
        .expect("test batch");

        let described = describe_batch(batch).expect("batch describes");
        assert_eq!(described.row_count, 2);
        assert_eq!(described.columns.len(), 3);

        let bool_col = &described.columns[0];
        assert!(
            bool_col
                .buffers
                .iter()
                .any(|buffer| buffer.role == BUFFER_VALIDITY)
        );
        let bool_values = bool_col
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("bool values");
        assert_eq!(bool_values.length, 1);
        assert_eq!(
            bool_values.address,
            bools.to_data().buffers()[0].as_ptr() as u64
        );

        assert!(
            described.columns[1]
                .buffers
                .iter()
                .all(|buffer| buffer.role != BUFFER_VALIDITY)
        );
        let number_values = described.columns[1]
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("number values");
        assert_eq!(number_values.length, 16);
        assert_eq!(
            number_values.address,
            numbers.to_data().buffers()[0].as_ptr() as u64
        );
        // SAFETY: address aliases the Int64Array values buffer retained by `numbers`.
        let first = unsafe { *(number_values.address as *const i64) };
        assert_eq!(first, 123);

        let string_col = &described.columns[2];
        assert!(
            string_col
                .buffers
                .iter()
                .any(|buffer| buffer.role == BUFFER_OFFSETS)
        );
        let string_values = string_col
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("string values");
        assert_eq!(
            string_values.address,
            strings.to_data().buffers()[1].as_ptr() as u64
        );
    }

    #[test]
    fn rejects_large_utf8() {
        let schema = Schema::new(vec![Field::new("s", DataType::LargeUtf8, true)]);
        let error = encode_schema(&schema).expect_err("large utf8 rejected");
        assert!(error.to_string().contains("large utf8"));
    }

    #[test]
    fn rejects_unsigned_integer_schema_and_batches() {
        let cases = [
            (
                DataType::UInt8,
                Arc::new(UInt8Array::from(vec![1u8])) as Arc<dyn Array>,
            ),
            (
                DataType::UInt16,
                Arc::new(UInt16Array::from(vec![1u16])) as Arc<dyn Array>,
            ),
            (
                DataType::UInt32,
                Arc::new(UInt32Array::from(vec![1u32])) as Arc<dyn Array>,
            ),
            (
                DataType::UInt64,
                Arc::new(UInt64Array::from(vec![1u64])) as Arc<dyn Array>,
            ),
        ];
        for (data_type, array) in cases {
            let schema = Schema::new(vec![Field::new("u", data_type.clone(), true)]);
            let schema_error = encode_schema(&schema).expect_err("unsigned schema rejected");
            assert!(
                matches!(
                    &schema_error,
                    ArrowStreamError::UnsupportedType { column, detail, .. }
                        if column == "u" && detail.contains("unsigned integer")
                ),
                "{schema_error}"
            );

            let batch_schema = Arc::new(schema);
            let batch = RecordBatch::try_new(Arc::clone(&batch_schema), vec![array])
                .expect("unsigned batch");
            let batch_error = describe_batch(batch).expect_err("unsigned batch rejected");
            assert!(
                matches!(
                    &batch_error,
                    ArrowStreamError::UnsupportedType { column, detail, .. }
                        if column == "u" && detail.contains("unsigned integer")
                ),
                "{batch_error}"
            );
        }
    }

    #[test]
    fn signed_int64_batch_still_exports_values_in_place() {
        let schema = Arc::new(Schema::new(vec![field("N", "FIXED", DataType::Int64)]));
        let numbers = Int64Array::from(vec![42i64]);
        let batch = RecordBatch::try_new(Arc::clone(&schema), vec![Arc::new(numbers.clone())])
            .expect("signed batch");
        let described = describe_batch(batch).expect("signed batch describes");
        let values = described.columns[0]
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("values");
        assert_eq!(
            values.address,
            numbers.to_data().buffers()[0].as_ptr() as u64
        );
        // SAFETY: address aliases the Int64Array values buffer retained by `numbers`.
        let stored = unsafe { *(values.address as *const i64) };
        assert_eq!(stored, 42);
    }

    #[test]
    fn rejects_non_byte_aligned_boolean_bit_offset() {
        let values = BooleanBuffer::new_set(9);
        let sliced = BooleanArray::new(values.slice(1, 8), None);
        assert_eq!(sliced.to_data().offset(), 1);
        let error = describe_array(&Field::new("flags", DataType::Boolean, true), &sliced)
            .expect_err("unaligned boolean rejected");
        assert!(
            matches!(
                &error,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "flags" && detail.contains("bit offset 1")
            ),
            "{error}"
        );
    }

    #[test]
    fn rejects_non_byte_aligned_validity_bit_offset() {
        let values = BooleanBuffer::new_set(9);
        let nulls = BooleanBuffer::new_unset(9);
        let sliced = BooleanArray::new(values.slice(1, 8), Some(nulls.slice(1, 8).into()));
        assert_eq!(sliced.to_data().offset(), 1);
        let error = describe_array(&Field::new("flags", DataType::Boolean, true), &sliced)
            .expect_err("unaligned validity rejected");
        assert!(
            matches!(
                &error,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "flags"
                        && (detail.contains("bit offset 1") || detail.contains("not byte-aligned"))
            ),
            "{error}"
        );
    }

    #[test]
    fn sliced_utf8_exports_logical_offset_slice_with_absolute_values_buffer() {
        const I32_BYTES: u64 = std::mem::size_of::<i32>() as u64;
        let full = StringArray::from(vec![Some("alpha"), Some("beta"), Some("gamma"), Some("d")]);
        let full_data = full.to_data();
        let full_offsets_ptr = full_data.buffers()[0].as_ptr() as u64;
        let full_values = full_data.buffers()[1].clone();
        // ArrayData::slice updates the logical offset in place rather than rewriting the offsets
        // buffer; describe_variable has to translate that to a byte offset of 1 * sizeof(i32).
        let sliced_data = full_data.slice(1, 2);
        assert_eq!(sliced_data.offset(), 1);
        let sliced = StringArray::from(sliced_data);

        let described = describe_array(&Field::new("s", DataType::Utf8, true), &sliced)
            .expect("sliced variable describes");

        let offsets = described
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_OFFSETS)
            .expect("offsets");
        let values = described
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("values");

        // Exports the slice [1, 2+1) of the i32 offsets, not a rewritten relative sequence.
        assert_eq!(offsets.length as u64, 3 * I32_BYTES);
        assert_eq!(offsets.address, full_offsets_ptr + I32_BYTES);
        // SAFETY: addresses alias `full`, which this test keeps alive.
        let last_offset = unsafe { *((offsets.address + 2 * I32_BYTES) as *const i32) } as usize;

        // Keeps the values buffer anchored at byte 0 so Arrow's absolute offsets stay valid.
        assert_eq!(values.address, full_values.as_ptr() as u64);
        assert!(last_offset <= values.length);
    }

    #[test]
    fn owns_and_drains_ffi_stream() {
        let schema = Arc::new(Schema::new(vec![field("B", "BOOLEAN", DataType::Boolean)]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(BooleanArray::from(vec![true]))],
        )
        .expect("test batch");
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let pointer = Box::into_raw(Box::new(FFI_ArrowArrayStream::new(Box::new(reader))));

        let streams = ArrowStreamRegistry::new();
        let handle = streams.open(pointer as i64).expect("stream opens");
        let schema_bytes = streams.schema(handle).expect("schema reads");
        assert_eq!(
            i32::from_le_bytes(schema_bytes[0..4].try_into().expect("fields")),
            1
        );
        assert!(read_batch(&streams, handle).expect("batch reads").is_some());
        assert!(read_batch(&streams, handle).expect("end reads").is_none());
        streams.close(handle);
        assert_eq!(streams.open_handles(), 0);
        assert!(matches!(
            read_batch(&streams, handle),
            Err(ArrowStreamError::MissingHandle { handle: missing, .. }) if missing == handle
        ));
    }

    #[test]
    fn open_rejects_null_pointer() {
        assert!(matches!(
            ArrowStreamRegistry::new().open(0),
            Err(ArrowStreamError::InvalidPointer { .. })
        ));
    }

    #[test]
    fn open_rejects_released_stream_and_frees_the_allocation() {
        let pointer = Box::into_raw(Box::new(FFI_ArrowArrayStream::empty()));
        let error = ArrowStreamRegistry::new()
            .open(pointer as i64)
            .expect_err("released stream");
        assert!(
            matches!(&error, ArrowStreamError::Arrow { source, .. } if source.to_string().contains("released")),
            "{error}"
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn failed_export_retries_the_same_batch() {
        let (schema, numbers, first) = int_batch(42);
        let expected = numbers.to_data().buffers()[0].as_ptr() as u64;
        let second = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Int64Array::from(vec![7i64]))],
        )
        .expect("second batch");
        let streams = ArrowStreamRegistry::new();
        let handle = open_reader(
            &streams,
            Box::new(RecordBatchIterator::new(
                vec![Ok(first), Ok(second)],
                schema,
            )),
        );

        let error = streams
            .export_next_batch(handle, |_| -> Result<(), String> {
                Err("export failed".to_string())
            })
            .expect_err("export fails");
        assert!(matches!(error, ArrowStreamError::Export { .. }));

        let retried = streams
            .export_next_batch(handle, |described| {
                Ok(value_address(described.expect("retried batch")))
            })
            .expect("retry exports");
        assert_eq!(retried, expected);

        let next = read_batch(&streams, handle)
            .expect("following batch")
            .expect("following batch is present");
        assert_ne!(value_address(&next), expected);
        streams.close(handle);
    }

    #[test]
    fn failed_describe_poisons_the_stream() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "flags",
            DataType::Boolean,
            true,
        )]));
        let values = BooleanBuffer::new_set(9);
        let bad = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(BooleanArray::new(values.slice(1, 8), None))],
        )
        .expect("unaligned batch");
        let good = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(BooleanArray::from(vec![true]))],
        )
        .expect("aligned batch");
        let streams = ArrowStreamRegistry::new();
        let handle = open_reader(
            &streams,
            Box::new(RecordBatchIterator::new(
                vec![Ok(bad), Ok(good)],
                Arc::clone(&schema),
            )),
        );

        let first = read_batch(&streams, handle).expect_err("unaligned batch");
        assert!(
            matches!(
                &first,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "flags" && detail.contains("bit offset 1")
            ),
            "{first}"
        );
        let second = read_batch(&streams, handle).expect_err("stream stays failed");
        assert!(
            matches!(
                &second,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "flags" && detail.contains("bit offset 1")
            ),
            "{second}"
        );
        streams.close(handle);
    }

    #[test]
    fn struct_child_error_uses_the_child_field_name() {
        let epoch = Field::new("epoch", DataType::UInt64, true);
        let field = Field::new(
            "ts",
            DataType::Struct(vec![Arc::new(epoch.clone())].into()),
            true,
        );
        let array = StructArray::try_new(
            vec![Arc::new(epoch)].into(),
            vec![Arc::new(UInt64Array::from(vec![1u64]))],
            None,
        )
        .expect("struct");
        let error = describe_array(&field, &array).expect_err("unsigned child");
        assert!(
            matches!(
                &error,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "epoch" && detail.contains("unsigned integer")
            ),
            "{error}"
        );
    }

    #[test]
    fn rejects_buffer_length_past_i32() {
        let error = byte_slice(
            "n",
            &arrow::buffer::Buffer::from(vec![1u8]),
            0,
            (i32::MAX as usize) + 1,
            BUFFER_VALUES,
        )
        .expect_err("length past i32");
        assert!(
            matches!(
                &error,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "n" && detail.contains("exceeds i32")
            ),
            "{error}"
        );
    }

    struct GateState {
        entered: bool,
        release: bool,
    }

    struct Gate {
        state: Mutex<GateState>,
        cv: Condvar,
    }

    impl Gate {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                state: Mutex::new(GateState {
                    entered: false,
                    release: false,
                }),
                cv: Condvar::new(),
            })
        }

        fn wait_until_entered(&self) {
            let mut state = self.state.lock().expect("gate");
            let deadline = Instant::now() + Duration::from_secs(2);
            while !state.entered {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "reader.next did not start");
                let (guard, timeout) = self.cv.wait_timeout(state, remaining).expect("gate");
                state = guard;
                if timeout.timed_out() && !state.entered {
                    panic!("reader.next did not start");
                }
            }
        }

        fn release(&self) {
            self.state.lock().expect("gate").release = true;
            self.cv.notify_all();
        }

        fn enter_and_wait(&self) {
            let mut state = self.state.lock().expect("gate");
            state.entered = true;
            self.cv.notify_all();
            while !state.release {
                state = self.cv.wait(state).expect("gate");
            }
        }
    }

    struct BlockingReader {
        schema: SchemaRef,
        batch: Option<RecordBatch>,
        gate: Arc<Gate>,
    }

    impl Iterator for BlockingReader {
        type Item = Result<RecordBatch, ArrowError>;

        fn next(&mut self) -> Option<Self::Item> {
            let batch = self.batch.take()?;
            let mut state = self.gate.state.lock().expect("gate");
            state.entered = true;
            self.gate.cv.notify_all();
            while !state.release {
                state = self.gate.cv.wait(state).expect("gate");
            }
            Some(Ok(batch))
        }
    }

    impl RecordBatchReader for BlockingReader {
        fn schema(&self) -> SchemaRef {
            Arc::clone(&self.schema)
        }
    }

    fn read_batch(
        streams: &ArrowStreamRegistry,
        handle: u64,
    ) -> Result<Option<BatchRef>, ArrowStreamError> {
        streams.export_next_batch(handle, |batch| Ok(batch.cloned()))
    }

    fn open_reader(
        streams: &ArrowStreamRegistry,
        reader: Box<dyn RecordBatchReader + Send>,
    ) -> u64 {
        let pointer = Box::into_raw(Box::new(FFI_ArrowArrayStream::new(reader)));
        streams.open(pointer as i64).expect("stream opens")
    }

    fn int_batch(value: i64) -> (SchemaRef, Int64Array, RecordBatch) {
        let schema = Arc::new(Schema::new(vec![field("N", "FIXED", DataType::Int64)]));
        let numbers = Int64Array::from(vec![value]);
        let batch = RecordBatch::try_new(Arc::clone(&schema), vec![Arc::new(numbers.clone())])
            .expect("test batch");
        (schema, numbers, batch)
    }

    fn value_address(batch: &BatchRef) -> u64 {
        batch.columns[0]
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("values")
            .address
    }

    #[test]
    fn next_batch_on_one_stream_does_not_block_another() {
        let streams = Arc::new(ArrowStreamRegistry::new());
        let gate = Gate::new();
        let (blocked_schema, _, blocked_batch) = int_batch(1);
        let blocked = open_reader(
            &streams,
            Box::new(BlockingReader {
                schema: blocked_schema,
                batch: Some(blocked_batch),
                gate: Arc::clone(&gate),
            }),
        );
        let (ready_schema, ready_numbers, ready_batch) = int_batch(7);
        let ready = open_reader(
            &streams,
            Box::new(RecordBatchIterator::new(
                vec![Ok(ready_batch)],
                ready_schema,
            )),
        );

        let blocked_streams = Arc::clone(&streams);
        let blocked_thread = thread::spawn(move || read_batch(&blocked_streams, blocked));
        gate.wait_until_entered();

        let (sender, receiver) = std::sync::mpsc::channel();
        let ready_streams = Arc::clone(&streams);
        let ready_thread = thread::spawn(move || {
            let _ = sender.send(read_batch(&ready_streams, ready));
        });
        let ready_result = receiver.recv_timeout(Duration::from_secs(2));
        gate.release();
        blocked_thread
            .join()
            .expect("blocked next_batch")
            .expect("batch reads");
        ready_thread.join().expect("ready next_batch");
        let ready_batch = ready_result
            .expect("second stream next_batch blocked behind the first")
            .expect("batch reads")
            .expect("batch is present");
        assert_eq!(
            value_address(&ready_batch),
            ready_numbers.to_data().buffers()[0].as_ptr() as u64
        );

        streams.close(blocked);
        streams.close(ready);
    }

    /// Runs `close` on its own thread and returns once it has finished, failing if it
    /// takes longer than a read blocked on a gate would.
    fn close_without_waiting(streams: &Arc<ArrowStreamRegistry>, handle: u64) {
        let finished = Arc::new(AtomicBool::new(false));
        let finished_flag = Arc::clone(&finished);
        let closing_streams = Arc::clone(streams);
        let closing = thread::spawn(move || {
            closing_streams.close(handle);
            finished_flag.store(true, Ordering::SeqCst);
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while !finished.load(Ordering::SeqCst) {
            assert!(
                Instant::now() < deadline,
                "close blocked behind the in-flight export"
            );
            thread::yield_now();
        }
        closing.join().expect("close");
    }

    #[test]
    fn close_detaches_from_in_flight_next_batch() {
        let streams = Arc::new(ArrowStreamRegistry::new());
        let gate = Gate::new();
        let (schema, _, batch) = int_batch(42);
        let handle = open_reader(
            &streams,
            Box::new(BlockingReader {
                schema,
                batch: Some(batch),
                gate: Arc::clone(&gate),
            }),
        );

        let reading_streams = Arc::clone(&streams);
        let reading = thread::spawn(move || read_batch(&reading_streams, handle));
        gate.wait_until_entered();

        // `reader.next` still holds the stream mutex. `close` must not wait for it.
        close_without_waiting(&streams, handle);

        gate.release();
        // The batch arrived after the stream was detached, so nothing retains its
        // buffers past this call and the read reports the handle as gone instead of
        // handing the addresses out.
        let read = reading.join().expect("next_batch");
        assert!(matches!(
            read,
            Err(ArrowStreamError::MissingHandle { handle: missing, .. }) if missing == handle
        ));
        assert!(matches!(
            read_batch(&streams, handle),
            Err(ArrowStreamError::MissingHandle { handle: missing, .. }) if missing == handle
        ));
    }

    #[test]
    fn close_detaches_during_buffer_export() {
        let streams = Arc::new(ArrowStreamRegistry::new());
        let gate = Gate::new();
        let (schema, numbers, batch) = int_batch(42);
        let expected = numbers.to_data().buffers()[0].as_ptr() as u64;
        drop(numbers);
        let handle = open_reader(
            &streams,
            Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema)),
        );
        let export_gate = Arc::clone(&gate);
        let exporting_streams = Arc::clone(&streams);
        let handles_while_exporting = Arc::new(AtomicUsize::new(usize::MAX));
        let observed_handles = Arc::clone(&handles_while_exporting);
        let exporting = thread::spawn(move || {
            exporting_streams.export_next_batch(handle, |described| {
                let address = value_address(described.expect("batch"));
                // SAFETY: `described` retains this buffer for the whole callback; a
                // concurrent `close` detaches the stream but does not drop the batch
                // the exporting thread still holds.
                let stored = unsafe { *(address as *const i64) };
                assert_eq!(stored, 42);
                assert_eq!(address, expected);
                export_gate.enter_and_wait();
                observed_handles.store(exporting_streams.open_handles(), Ordering::SeqCst);
                Ok(())
            })
        });
        gate.wait_until_entered();

        // The export callback still holds the stream mutex. `close` must not wait.
        close_without_waiting(&streams, handle);

        gate.release();
        // The callback succeeded, but the export still fails: after `close` nothing
        // keeps the exported addresses alive, so reporting success would hand the
        // caller a dangling batch.
        assert!(matches!(
            exporting.join().expect("export"),
            Err(ArrowStreamError::MissingHandle { handle: missing, .. }) if missing == handle
        ));
        // `close` ran to completion while the export held the stream mutex and left the
        // registry entry behind: that entry, not the exporting thread's `Arc`, is what
        // keeps a batch alive across the end of an export that does publish one.
        assert_eq!(handles_while_exporting.load(Ordering::SeqCst), 1);
        // The export that observed `closing` owns the removal `close` skipped.
        assert_eq!(streams.open_handles(), 0);
        assert!(matches!(
            read_batch(&streams, handle),
            Err(ArrowStreamError::MissingHandle { handle: missing, .. }) if missing == handle
        ));
    }

    const I64_BYTES: usize = std::mem::size_of::<i64>();

    fn child_values_buffer(described: &ArrayRef) -> &BufferRef {
        described.children[0]
            .buffers
            .iter()
            .find(|buffer| buffer.role == BUFFER_VALUES)
            .expect("child values")
    }

    #[test]
    fn sliced_struct_describes_children_from_the_slice_origin() {
        let child = Int64Array::from(vec![10, 20, 30, 40]);
        let child_values = child.to_data().buffers()[0].as_ptr() as u64;
        let child_field = Arc::new(field("N", "FIXED", DataType::Int64));
        let full = StructArray::from(vec![(
            Arc::clone(&child_field),
            Arc::new(child) as Arc<dyn Array>,
        )]);
        let sliced = full.slice(2, 2);
        // A `StructArray` carries no logical offset of its own: both `slice` and
        // `From<ArrayData>` push the parent offset down into the children, so
        // `describe_struct` only ever sees offset 0 and the children are already
        // positioned. `slice_child` below is the no-op that keeps that true.
        assert_eq!(sliced.to_data().offset(), 0);

        let struct_field = Field::new("S", DataType::Struct(vec![child_field].into()), true);
        let described = describe_array(&struct_field, &sliced).expect("sliced struct describes");

        let values = child_values_buffer(&described);
        assert_eq!(values.length, 2 * I64_BYTES);
        assert_eq!(values.address, child_values + 2 * I64_BYTES as u64);
        // SAFETY: the address aliases `full`, which this test keeps alive.
        assert_eq!(unsafe { *(values.address as *const i64) }, 30);
    }

    #[test]
    fn sliced_fixed_size_list_describes_the_child_scaled_by_the_list_width() {
        const LIST_SIZE: i32 = 3;
        let child_field = Arc::new(field("E", "FIXED", DataType::Int64));
        let child = Int64Array::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let child_values = child.to_data().buffers()[0].as_ptr() as u64;
        let full = FixedSizeListArray::new(
            Arc::clone(&child_field),
            LIST_SIZE,
            Arc::new(child) as Arc<dyn Array>,
            None,
        );
        let sliced = full.slice(1, 2);
        // Same invariant as the struct above, scaled by the list width: the child was
        // already narrowed to [1 * LIST_SIZE, 3 * LIST_SIZE).
        assert_eq!(sliced.to_data().offset(), 0);

        let list_field = Field::new(
            "V",
            DataType::FixedSizeList(Arc::clone(&child_field), LIST_SIZE),
            true,
        );
        let described = describe_array(&list_field, &sliced).expect("sliced list describes");

        let values = child_values_buffer(&described);
        assert_eq!(values.length, 6 * I64_BYTES);
        assert_eq!(values.address, child_values + 3 * I64_BYTES as u64);
        // SAFETY: the address aliases `full`, which this test keeps alive.
        assert_eq!(unsafe { *(values.address as *const i64) }, 4);
    }

    #[test]
    fn slice_child_rejects_a_range_past_the_child() {
        let child = Int64Array::from(vec![1, 2]).to_data();
        // `ArrayData::slice` panics on an out-of-range slice. A nested array whose
        // declared length runs past its child must surface the column name instead,
        // because a panic here would unwind across the JNI boundary.
        let error = slice_child("S", &child, 1, 4).expect_err("range past the child");
        assert!(
            matches!(
                &error,
                ArrowStreamError::UnsupportedType { column, detail, .. }
                    if column == "S" && detail.contains("exceeds child length")
            ),
            "{error}"
        );
    }
}
