use crate::error::{BridgeError, ToJsError};
use napi::Env;
use napi::bindgen_prelude::{
    Either, FnArgs, FromNapiValue, Function, JsObjectValue, JsValuesTupleIntoVec, Object, Promise,
    TypeName, ValidateNapiValue, ValueType, block_on,
};
use napi::sys;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use sf_core::token_cache::{
    CacheKey, KeyringTokenCache, TokenCache, TokenCacheError, TokenRemovalSnafu,
    TokenRetrievalSnafu, TokenStorageSnafu, build_cache_key,
};
use sf_core::utils::sync::RwLockRecoverExt;
use snafu::ResultExt;
use std::future::Future;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);

type JsCallback<Args, Ret> =
    ThreadsafeFunction<Args, Either<Promise<Ret>, Ret>, Args, napi::Status, false, true>;

struct Ignored;

impl TypeName for Ignored {
    fn type_name() -> &'static str {
        "unknown"
    }

    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl ValidateNapiValue for Ignored {
    unsafe fn validate(_: sys::napi_env, _: sys::napi_value) -> napi::Result<sys::napi_value> {
        Ok(std::ptr::null_mut())
    }
}

impl FromNapiValue for Ignored {
    unsafe fn from_napi_value(_: sys::napi_env, _: sys::napi_value) -> napi::Result<Self> {
        Ok(Ignored)
    }
}

struct CredentialManagerCallbacks {
    read: JsCallback<FnArgs<(String,)>, Option<String>>,
    write: JsCallback<FnArgs<(String, String)>, Ignored>,
    remove: JsCallback<FnArgs<(String,)>, Ignored>,
}

impl CredentialManagerCallbacks {
    fn read(&self, key: String) -> napi::Result<Option<String>> {
        call(&self.read, FnArgs::from((key,)))
    }

    fn write(&self, key: String, token: String) -> napi::Result<()> {
        call(&self.write, FnArgs::from((key, token))).map(drop)
    }

    fn remove(&self, key: String) -> napi::Result<()> {
        call(&self.remove, FnArgs::from((key,))).map(drop)
    }
}

fn call<Args, Ret>(callback: &JsCallback<Args, Ret>, args: Args) -> napi::Result<Ret>
where
    Args: JsValuesTupleIntoVec + 'static,
    Ret: FromNapiValue + TypeName + ValidateNapiValue + 'static,
{
    wait_for_callback(async {
        match callback.call_async_catch(args).await? {
            Either::A(promise) => promise.await,
            Either::B(value) => Ok(value),
        }
    })
}

fn wait_for_callback<T>(future: impl Future<Output = napi::Result<T>>) -> napi::Result<T> {
    block_on(async {
        match tokio::time::timeout(CALLBACK_TIMEOUT, future).await {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    timeout_secs = CALLBACK_TIMEOUT.as_secs(),
                    "Custom credential manager callback timed out"
                );
                Err(napi::Error::new(
                    napi::Status::Cancelled,
                    "Custom credential manager callback timed out",
                ))
            }
        }
    })
}

pub(crate) struct NodeTokenCache {
    callbacks: RwLock<Option<Arc<CredentialManagerCallbacks>>>,
    fallback: OnceLock<Arc<dyn TokenCache>>,
}

impl NodeTokenCache {
    pub(crate) fn new() -> Self {
        Self {
            callbacks: RwLock::new(None),
            fallback: OnceLock::new(),
        }
    }

    fn callbacks(&self) -> Option<Arc<CredentialManagerCallbacks>> {
        self.callbacks.read_recover().clone()
    }

    fn fallback(&self) -> Result<Arc<dyn TokenCache>, TokenCacheError> {
        if let Some(cache) = self.fallback.get() {
            return Ok(Arc::clone(cache));
        }
        let cache: Arc<dyn TokenCache> = Arc::new(KeyringTokenCache::new()?);
        Ok(Arc::clone(self.fallback.get_or_init(|| cache)))
    }

    fn set_callbacks(&self, callbacks: Option<CredentialManagerCallbacks>) {
        *self.callbacks.write_recover() = callbacks.map(Arc::new);
    }
}

impl TokenCache for NodeTokenCache {
    fn add_token(&self, key: &CacheKey, token_value: &str) -> Result<(), TokenCacheError> {
        let Some(callbacks) = self.callbacks() else {
            return self.fallback()?.add_token(key, token_value);
        };
        callbacks
            .write(build_cache_key(key), token_value.to_string())
            .boxed()
            .context(TokenStorageSnafu)
    }

    fn remove_token(&self, key: &CacheKey) -> Result<(), TokenCacheError> {
        let Some(callbacks) = self.callbacks() else {
            return self.fallback()?.remove_token(key);
        };
        callbacks
            .remove(build_cache_key(key))
            .boxed()
            .context(TokenRemovalSnafu)
    }

    fn get_token(&self, key: &CacheKey) -> Result<Option<String>, TokenCacheError> {
        let Some(callbacks) = self.callbacks() else {
            return self.fallback()?.get_token(key);
        };
        callbacks
            .read(build_cache_key(key))
            .boxed()
            .context(TokenRetrievalSnafu)
    }
}

/// Token store; methods may return a value or a Promise that settles within 30 seconds.
#[napi(object, object_to_js = false)]
pub struct CustomCredentialManager {
    #[napi(
        ts_type = "(key: string) => string | null | undefined | Promise<string | null | undefined>"
    )]
    pub read: (),
    #[napi(ts_type = "(key: string, token: string) => unknown")]
    pub write: (),
    #[napi(ts_type = "(key: string) => unknown")]
    pub remove: (),
}

#[napi(ts_args_type = "manager: CustomCredentialManager | null")]
pub fn set_custom_credential_manager(env: Env, manager: Option<Object>) -> napi::Result<()> {
    let callbacks = manager
        .map(|manager| -> napi::Result<_> {
            Ok(CredentialManagerCallbacks {
                read: threadsafe(manager, "read")?,
                write: threadsafe(manager, "write")?,
                remove: threadsafe(manager, "remove")?,
            })
        })
        .transpose()
        .map_err(|e| BridgeError::InvalidParameter(e.reason.clone()).to_js_error(env, None))?;
    crate::BRIDGE.token_cache.set_callbacks(callbacks);
    Ok(())
}

fn threadsafe<Args, Ret>(manager: Object<'_>, method: &str) -> napi::Result<JsCallback<Args, Ret>>
where
    Args: JsValuesTupleIntoVec + 'static,
    Ret: FromNapiValue + TypeName + ValidateNapiValue + 'static,
{
    let function: Function<'_, Args, Either<Promise<Ret>, Ret>> =
        manager.get_named_property(method)?;
    function
        .bind(manager)?
        .build_threadsafe_function()
        .weak::<true>()
        .build()
}
