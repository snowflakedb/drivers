use jni::JNIEnv;
use jni::objects::{AsJArrayRaw, JByteArray, JClass, JLongArray, JObject, JString, JValue};
use jni::sys::{jbyteArray, jint, jlongArray, jobject};
use proto_utils::ProtoError;
use sf_core::c_api::{
    CApiHandle, SfCoreApi, sf_core_api_destroy, sf_core_api_flush, sf_core_api_init,
    sf_core_api_read, sf_core_api_write,
};
use sf_core::protobuf_apis::call_proto;

mod slf4j_layer;

/// Convert a CApiHandle to a Java long for storage in Java objects
fn handle_to_jlong_array<'a>(env: &mut JNIEnv<'a>, handle: CApiHandle) -> JLongArray<'a> {
    // Store the handle as two 64-bit values in a 2-element array
    // First element: id as a full 64-bit value, Second element: magic as a full 64-bit value
    // Return java struct
    let array = env.new_long_array(2).unwrap();
    env.set_long_array_region(&array, 0, &[handle.id as i64, handle.magic as i64])
        .unwrap();
    array
}

/// Convert a Java long back to a CApiHandle
fn jlong_array_to_handle<'a>(env: &mut JNIEnv<'a>, array: JLongArray<'a>) -> CApiHandle {
    let mut buffer = [0i64; 2];
    env.get_long_array_region(&array, 0, &mut buffer).unwrap();
    CApiHandle {
        id: buffer[0] as u64,
        magic: buffer[1] as u64,
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn JNI_OnLoad(jvm: *mut jni::sys::JavaVM, _: *mut u8) -> jint {
    let config = sf_core::logging::LoggingConfig::new(None, false, false);
    let layer = slf4j_layer::SLF4JLayer::new(jvm);
    match sf_core::logging::init_logging(config, Some(layer)) {
        Ok(_) => jni::sys::JNI_VERSION_1_2,
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn JNI_OnUnload(_jvm: *mut jni::sys::JavaVM, _: *mut u8) -> jint {
    0
}

/// Initialize the sf_core API
///
/// # Arguments
/// * `_env` - JNI environment
/// * `_class` - The calling Java class
/// * `api_type` - The API type (1 for DatabaseDriverApiV1)
///
/// # Returns
/// A handle to the API instance as a Java long
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_snowflake_jdbc_CoreTransport_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    api_type: jint,
) -> jlongArray {
    let api = match api_type {
        1 => SfCoreApi::DatabaseDriverApiV1,
        _ => {
            // Return 0 to indicate failure
            todo!()
        }
    };

    let handle = sf_core_api_init(api);
    handle_to_jlong_array(&mut env, handle).as_jarray_raw() as jlongArray
}

/// Destroy the sf_core API instance
///
/// # Arguments
/// * `_env` - JNI environment
/// * `_class` - The calling Java class
/// * `handle` - The API handle to destroy
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_snowflake_jdbc_CoreTransport_nativeDestroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlongArray,
) {
    let api_handle = jlong_array_to_handle(&mut env, unsafe { JLongArray::from_raw(handle) });
    sf_core_api_destroy(api_handle);
}

/// Write data to the transport
///
/// # Arguments
/// * `env` - JNI environment
/// * `_class` - The calling Java class
/// * `handle` - The API handle
/// * `buffer` - The Java byte array containing data to write
/// * `length` - The number of bytes to write
///
/// # Returns
/// The number of bytes written
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_snowflake_jdbc_CoreTransport_nativeWrite(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlongArray,
    buffer: jbyteArray,
    length: jint,
) -> jint {
    if length < 0 {
        return -1;
    }

    let api_handle = jlong_array_to_handle(&mut env, unsafe { JLongArray::from_raw(handle) });
    let length_usize = length as usize;

    // Get the Java byte array
    let java_buffer = unsafe { JByteArray::from_raw(buffer) };

    // Convert Java byte array to Rust Vec<u8>
    let rust_buffer = match env.convert_byte_array(&java_buffer) {
        Ok(buffer) => buffer,
        Err(_) => return -1,
    };

    // Ensure we don't write more bytes than available
    let bytes_to_write = std::cmp::min(length_usize, rust_buffer.len());

    if bytes_to_write == 0 {
        return 0;
    }

    // Call the sf_core API
    let bytes_written =
        unsafe { sf_core_api_write(api_handle, rust_buffer.as_ptr() as *mut u8, bytes_to_write) };

    bytes_written as jint
}

/// Read data from the transport
///
/// # Arguments
/// * `env` - JNI environment
/// * `_class` - The calling Java class
/// * `handle` - The API handle
/// * `buffer` - The Java byte array to read into
/// * `length` - The maximum number of bytes to read
///
/// # Returns
/// The number of bytes read
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_snowflake_jdbc_CoreTransport_nativeRead(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlongArray,
    buffer: jbyteArray,
    length: jint,
) -> jint {
    if length < 0 {
        return -1;
    }

    let api_handle = jlong_array_to_handle(&mut env, unsafe { JLongArray::from_raw(handle) });
    let length_usize = length as usize;

    if length_usize == 0 {
        return 0;
    }

    // Get the Java byte array
    let java_buffer = unsafe { JByteArray::from_raw(buffer) };

    // Create a temporary buffer for reading
    let mut read_buffer = vec![0i8; length_usize];

    // Call the sf_core API
    let bytes_read = unsafe {
        sf_core_api_read(
            api_handle,
            read_buffer.as_mut_ptr() as *mut u8,
            length_usize,
        )
    };

    if bytes_read > 0 {
        // Copy the read data back to the Java byte array
        let bytes_to_copy = std::cmp::min(bytes_read, length_usize);
        if env
            .set_byte_array_region(&java_buffer, 0, &read_buffer[..bytes_to_copy] as &[i8])
            .is_err()
        {
            return -1;
        }
    }

    bytes_read as jint
}

/// Flush the transport
///
/// # Arguments
/// * `_env` - JNI environment
/// * `_class` - The calling Java class
/// * `handle` - The API handle
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_snowflake_jdbc_CoreTransport_nativeFlush(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlongArray,
) {
    let api_handle = jlong_array_to_handle(&mut env, unsafe { JLongArray::from_raw(handle) });
    sf_core_api_flush(api_handle);
}

/// Handle a protobuf message
///
/// # Arguments
/// * `env` - JNI environment
/// * `_class` - The calling Java class
/// * `service_name` - The service name
/// * `method_name` - The method name
/// * `request_bytes` - The request bytes
///
/// # Returns
/// A TransportResponse object containing the status code and response bytes
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_snowflake_unicore_JNICoreTransport_nativeHandleMessage(
    mut env: JNIEnv,
    _class: JClass,
    service_name: JString,
    method_name: JString,
    request_bytes: JByteArray,
) -> jobject {
    // Convert Java strings and byte array to Rust types
    let service_name_str = match env.get_string(&service_name) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let method_name_str = match env.get_string(&method_name) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let request_bytes_vec = match env.convert_byte_array(&request_bytes) {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };

    // Call the protobuf API
    let result = call_proto(
        &service_name_str.to_string_lossy(),
        &method_name_str.to_string_lossy(),
        request_bytes_vec.as_slice(),
    );

    // Find the TransportResponse class
    let response_class =
        match env.find_class("com/snowflake/unicore/CoreTransport$TransportResponse") {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };

    // Create the appropriate response based on the result
    let response_obj = match result {
        Ok(response) => {
            // Success case - code 0
            let response_array = match env.byte_array_from_slice(&response) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[
                    JValue::Int(0),
                    JValue::Object(&JObject::from(response_array)),
                ],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(ProtoError::Application(error)) => {
            // Application error - code 1
            let error_array = match env.byte_array_from_slice(&error) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[JValue::Int(1), JValue::Object(&JObject::from(error_array))],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(ProtoError::Transport(error_msg)) => {
            // Transport error - code 2
            let error_array = match env.byte_array_from_slice(error_msg.as_bytes()) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[JValue::Int(2), JValue::Object(&JObject::from(error_array))],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
    };

    response_obj.into_raw()
}
