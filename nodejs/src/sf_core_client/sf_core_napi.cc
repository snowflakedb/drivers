#include <dlfcn.h>
#include <napi.h>

#include <cstring>
#include <mutex>
#include <string>

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

static Napi::Object Init(Napi::Env env, Napi::Object exports) {
  exports.Set("apiCallProto", Napi::Function::New(env, ApiCallProto));
  exports.Set("initLogger", Napi::Function::New(env, InitLogger));
  return exports;
}

NODE_API_MODULE(sf_core_napi, Init)
