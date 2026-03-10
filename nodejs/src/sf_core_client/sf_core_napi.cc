#include <dlfcn.h>
#include <napi.h>

#include <cstring>
#include <map>
#include <mutex>
#include <string>
#include <vector>

#include "arrow_abi.h"

// sf_core C function signatures
typedef size_t (*sf_core_api_call_proto_fn)(const char* api, const char* method, const uint8_t* request,
                                            size_t request_len, const uint8_t** response, size_t* response_len);

typedef void (*sf_core_free_buffer_fn)(const uint8_t* buffer, size_t len);

typedef uint32_t (*sf_core_log_callback_fn)(uint32_t level, const char* message, const char* filename, uint32_t line,
                                            const char* function);

typedef uint32_t (*sf_core_init_logger_fn)(sf_core_log_callback_fn callback);

// Resolved function pointers
static sf_core_api_call_proto_fn fn_api_call_proto = nullptr;
static sf_core_free_buffer_fn fn_free_buffer = nullptr;
static sf_core_init_logger_fn fn_init_logger = nullptr;
static void* lib_handle = nullptr;

// Thread-safe function for logger callback
static Napi::ThreadSafeFunction tsfn;
static bool logger_initialized = false;

static bool LoadLibrary() {
  if (lib_handle) return true;

  // Hardcoded path for arm64 macOS debug build
  const char* lib_path = "../target/debug/libsf_core.dylib";

  lib_handle = dlopen(lib_path, RTLD_LAZY);
  if (!lib_handle) {
    return false;
  }

  fn_api_call_proto = (sf_core_api_call_proto_fn)dlsym(lib_handle, "sf_core_api_call_proto");
  fn_free_buffer = (sf_core_free_buffer_fn)dlsym(lib_handle, "sf_core_free_buffer");
  fn_init_logger = (sf_core_init_logger_fn)dlsym(lib_handle, "sf_core_init_logger");

  if (!fn_api_call_proto || !fn_free_buffer || !fn_init_logger) {
    dlclose(lib_handle);
    lib_handle = nullptr;
    return false;
  }

  return true;
}

// apiCallProto(api: string, method: string, request: Buffer): { code: number, data: Buffer }
static Napi::Value ApiCallProto(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 3) {
    Napi::TypeError::New(env, "Expected 3 arguments: api, method, request").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  if (!LoadLibrary()) {
    Napi::Error::New(env, "Failed to load libsf_core.dylib. Build it first: cargo build --package sf_core")
        .ThrowAsJavaScriptException();
    return env.Undefined();
  }

  std::string api = info[0].As<Napi::String>().Utf8Value();
  std::string method = info[1].As<Napi::String>().Utf8Value();
  Napi::Buffer<uint8_t> request_buf = info[2].As<Napi::Buffer<uint8_t>>();

  // sf_core requires a non-null pointer even for empty messages
  static uint8_t empty_byte = 0;
  const uint8_t* request_data = request_buf.Length() > 0 ? request_buf.Data() : &empty_byte;
  size_t request_len = request_buf.Length();

  const uint8_t* response = nullptr;
  size_t response_len = 0;

  size_t code = fn_api_call_proto(api.c_str(), method.c_str(), request_data, request_len, &response, &response_len);

  // Copy response into a JS Buffer before freeing
  Napi::Buffer<uint8_t> result_data = Napi::Buffer<uint8_t>::Copy(env, response, response_len);

  if (response) {
    fn_free_buffer(response, response_len);
  }

  Napi::Object result = Napi::Object::New(env);
  result.Set("code", Napi::Number::New(env, static_cast<double>(code)));
  result.Set("data", result_data);

  return result;
}

// Static C callback that forwards to JS via ThreadSafeFunction
static uint32_t LoggerCallback(uint32_t level, const char* message, const char* filename, uint32_t line,
                               const char* function) {
  if (!logger_initialized) return 0;

  struct LogData {
    uint32_t level;
    std::string message;
    std::string filename;
    uint32_t line;
    std::string function;
  };

  auto* data = new LogData{level, message ? message : "", filename ? filename : "", line, function ? function : ""};

  tsfn.NonBlockingCall(data, [](Napi::Env env, Napi::Function jsCallback, LogData* d) {
    jsCallback.Call({Napi::Number::New(env, d->level), Napi::String::New(env, d->message),
                     Napi::String::New(env, d->filename), Napi::Number::New(env, d->line),
                     Napi::String::New(env, d->function)});
    delete d;
  });

  return 0;
}

// initLogger(callback: (level, message, filename, line, func) => void): number
static Napi::Value InitLogger(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsFunction()) {
    Napi::TypeError::New(env, "Expected a callback function").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  if (!LoadLibrary()) {
    Napi::Error::New(env, "Failed to load libsf_core.dylib").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  if (logger_initialized) {
    return Napi::Number::New(env, 0);
  }

  Napi::Function jsCallback = info[0].As<Napi::Function>();

  tsfn = Napi::ThreadSafeFunction::New(env, jsCallback, "sf_core_logger",
                                       0,  // unlimited queue
                                       1   // one initial thread
  );

  // Allow Node.js to exit even if the logger callback is still registered
  tsfn.Unref(env);

  logger_initialized = true;

  uint32_t result = fn_init_logger(LoggerCallback);
  return Napi::Number::New(env, result);
}

// ── Arrow stream reader ─────────────────────────────────────────────────────

static uint32_t next_stream_handle = 1;
static std::map<uint32_t, ArrowArrayStream*> open_streams;

static void ReleaseSchemaRecursive(ArrowSchema* schema) {
  if (schema && schema->release) {
    schema->release(schema);
  }
}

static void ReleaseArray(ArrowArray* array) {
  if (array && array->release) {
    array->release(array);
  }
}

static std::vector<std::string> ExtractColumnNames(ArrowSchema* schema) {
  std::vector<std::string> names;
  for (int64_t i = 0; i < schema->n_children; i++) {
    const char* name = schema->children[i]->name;
    names.push_back(name ? name : "");
  }
  return names;
}

// Read a single value from an Arrow array column and set it on a JS object.
// Supports the common types needed for the POC; extend as needed.
static void SetValueFromArrow(Napi::Env env, Napi::Object& row, const std::string& col_name, ArrowSchema* col_schema,
                              ArrowArray* col_array, int64_t row_idx) {
  int64_t actual_idx = row_idx + col_array->offset;

  // Check for null via validity bitmap (buffer 0)
  if (col_array->null_count != 0 && col_array->buffers[0] != nullptr) {
    const uint8_t* validity = static_cast<const uint8_t*>(col_array->buffers[0]);
    if (!(validity[actual_idx / 8] & (1 << (actual_idx % 8)))) {
      row.Set(col_name, env.Null());
      return;
    }
  }

  const char* format = col_schema->format;

  // "l" = int64
  if (format[0] == 'l' && format[1] == '\0') {
    const int64_t* data = static_cast<const int64_t*>(col_array->buffers[1]);
    int64_t val = data[actual_idx];
    // Use Number for values that fit safely; BigInt otherwise
    if (val >= -9007199254740991LL && val <= 9007199254740991LL) {
      row.Set(col_name, Napi::Number::New(env, static_cast<double>(val)));
    } else {
      row.Set(col_name, Napi::BigInt::New(env, val));
    }
    return;
  }

  // "i" = int32
  if (format[0] == 'i' && format[1] == '\0') {
    const int32_t* data = static_cast<const int32_t*>(col_array->buffers[1]);
    row.Set(col_name, Napi::Number::New(env, data[actual_idx]));
    return;
  }

  // "s" = int16
  if (format[0] == 's' && format[1] == '\0') {
    const int16_t* data = static_cast<const int16_t*>(col_array->buffers[1]);
    row.Set(col_name, Napi::Number::New(env, data[actual_idx]));
    return;
  }

  // "c" = int8
  if (format[0] == 'c' && format[1] == '\0') {
    const int8_t* data = static_cast<const int8_t*>(col_array->buffers[1]);
    row.Set(col_name, Napi::Number::New(env, data[actual_idx]));
    return;
  }

  // "g" = float64 (double)
  if (format[0] == 'g' && format[1] == '\0') {
    const double* data = static_cast<const double*>(col_array->buffers[1]);
    row.Set(col_name, Napi::Number::New(env, data[actual_idx]));
    return;
  }

  // "f" = float32
  if (format[0] == 'f' && format[1] == '\0') {
    const float* data = static_cast<const float*>(col_array->buffers[1]);
    row.Set(col_name, Napi::Number::New(env, data[actual_idx]));
    return;
  }

  // "b" = boolean
  if (format[0] == 'b' && format[1] == '\0') {
    const uint8_t* data = static_cast<const uint8_t*>(col_array->buffers[1]);
    bool val = data[actual_idx / 8] & (1 << (actual_idx % 8));
    row.Set(col_name, Napi::Boolean::New(env, val));
    return;
  }

  // "u" = utf8 string (variable-length, 32-bit offsets)
  if (format[0] == 'u' && format[1] == '\0') {
    const int32_t* offsets = static_cast<const int32_t*>(col_array->buffers[1]);
    const char* data = static_cast<const char*>(col_array->buffers[2]);
    int32_t start = offsets[actual_idx];
    int32_t end = offsets[actual_idx + 1];
    row.Set(col_name, Napi::String::New(env, data + start, end - start));
    return;
  }

  // "U" = large utf8 string (variable-length, 64-bit offsets)
  if (format[0] == 'U' && format[1] == '\0') {
    const int64_t* offsets = static_cast<const int64_t*>(col_array->buffers[1]);
    const char* data = static_cast<const char*>(col_array->buffers[2]);
    int64_t start = offsets[actual_idx];
    int64_t end = offsets[actual_idx + 1];
    row.Set(col_name, Napi::String::New(env, data + start, static_cast<size_t>(end - start)));
    return;
  }

  // "n" = null type
  if (format[0] == 'n' && format[1] == '\0') {
    row.Set(col_name, env.Null());
    return;
  }

  // Unsupported type: return as string describing the format
  std::string fallback = std::string("[unsupported Arrow type: ") + format + "]";
  row.Set(col_name, Napi::String::New(env, fallback));
}

// openArrowStream(pointerBuffer: Buffer): { handle: number, columnNames: string[] }
static Napi::Value OpenArrowStream(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsBuffer()) {
    Napi::TypeError::New(env, "Expected a Buffer containing the ArrowArrayStream pointer").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  Napi::Buffer<uint8_t> buf = info[0].As<Napi::Buffer<uint8_t>>();
  if (buf.Length() != sizeof(void*)) {
    Napi::TypeError::New(env, "Pointer buffer must be exactly sizeof(void*) bytes").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  ArrowArrayStream* stream = nullptr;
  std::memcpy(&stream, buf.Data(), sizeof(void*));

  if (!stream || !stream->release) {
    Napi::Error::New(env, "Invalid or already-released ArrowArrayStream pointer").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  // Get schema to extract column names
  ArrowSchema schema;
  std::memset(&schema, 0, sizeof(schema));
  int rc = stream->get_schema(stream, &schema);
  if (rc != 0) {
    const char* err = stream->get_last_error(stream);
    std::string msg = "get_schema failed: ";
    msg += err ? err : "unknown error";
    Napi::Error::New(env, msg).ThrowAsJavaScriptException();
    return env.Undefined();
  }

  std::vector<std::string> col_names = ExtractColumnNames(&schema);
  ReleaseSchemaRecursive(&schema);

  uint32_t handle = next_stream_handle++;
  open_streams[handle] = stream;

  Napi::Object result = Napi::Object::New(env);
  result.Set("handle", Napi::Number::New(env, handle));

  Napi::Array names_arr = Napi::Array::New(env, col_names.size());
  for (size_t i = 0; i < col_names.size(); i++) {
    names_arr.Set(static_cast<uint32_t>(i), Napi::String::New(env, col_names[i]));
  }
  result.Set("columnNames", names_arr);

  return result;
}

// readNextBatch(handle: number): object[] | null
static Napi::Value ReadNextBatch(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsNumber()) {
    Napi::TypeError::New(env, "Expected a stream handle (number)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  uint32_t handle = info[0].As<Napi::Number>().Uint32Value();
  auto it = open_streams.find(handle);
  if (it == open_streams.end()) {
    Napi::Error::New(env, "Unknown or closed stream handle").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  ArrowArrayStream* stream = it->second;

  // We need the schema to know column formats
  ArrowSchema schema;
  std::memset(&schema, 0, sizeof(schema));
  int rc = stream->get_schema(stream, &schema);
  if (rc != 0) {
    const char* err = stream->get_last_error(stream);
    std::string msg = "get_schema failed: ";
    msg += err ? err : "unknown error";
    Napi::Error::New(env, msg).ThrowAsJavaScriptException();
    return env.Undefined();
  }

  ArrowArray batch;
  std::memset(&batch, 0, sizeof(batch));
  rc = stream->get_next(stream, &batch);
  if (rc != 0) {
    const char* err = stream->get_last_error(stream);
    std::string msg = "get_next failed: ";
    msg += err ? err : "unknown error";
    ReleaseSchemaRecursive(&schema);
    Napi::Error::New(env, msg).ThrowAsJavaScriptException();
    return env.Undefined();
  }

  // End of stream
  if (!batch.release) {
    ReleaseSchemaRecursive(&schema);
    return env.Null();
  }

  // Convert batch to array of JS objects
  int64_t n_rows = batch.length;
  int64_t n_cols = schema.n_children;

  Napi::Array rows = Napi::Array::New(env, static_cast<size_t>(n_rows));

  for (int64_t r = 0; r < n_rows; r++) {
    Napi::Object row = Napi::Object::New(env);
    for (int64_t c = 0; c < n_cols; c++) {
      const char* name = schema.children[c]->name;
      std::string col_name = name ? name : "";
      SetValueFromArrow(env, row, col_name, schema.children[c], batch.children[c], r);
    }
    rows.Set(static_cast<uint32_t>(r), row);
  }

  ReleaseArray(&batch);
  ReleaseSchemaRecursive(&schema);

  return rows;
}

// closeArrowStream(handle: number): void
static Napi::Value CloseArrowStream(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsNumber()) {
    Napi::TypeError::New(env, "Expected a stream handle (number)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  uint32_t handle = info[0].As<Napi::Number>().Uint32Value();
  auto it = open_streams.find(handle);
  if (it == open_streams.end()) {
    return env.Undefined();
  }

  ArrowArrayStream* stream = it->second;
  if (stream && stream->release) {
    stream->release(stream);
  }
  open_streams.erase(it);

  return env.Undefined();
}

static Napi::Object Init(Napi::Env env, Napi::Object exports) {
  exports.Set("apiCallProto", Napi::Function::New(env, ApiCallProto));
  exports.Set("initLogger", Napi::Function::New(env, InitLogger));
  exports.Set("openArrowStream", Napi::Function::New(env, OpenArrowStream));
  exports.Set("readNextBatch", Napi::Function::New(env, ReadNextBatch));
  exports.Set("closeArrowStream", Napi::Function::New(env, CloseArrowStream));
  return exports;
}

NODE_API_MODULE(sf_core_napi, Init)
