use napi::bindgen_prelude::{
    Either, FnArgs, Function, FunctionRef, JsObjectValue, JsValuesTupleIntoVec, Object, Unknown,
};
use napi::{Env, Error, Result};
use napi_derive::napi;

pub(crate) type SnowflakeDateArgs = FnArgs<(f64, u32, u32, Either<String, i32>, String)>;

/// JavaScript classes the bridge instantiates when it materializes cells.
pub(crate) struct TypeConstructors {
    pub snowflake_date: FunctionRef<SnowflakeDateArgs, Unknown<'static>>,
}

impl TypeConstructors {
    pub(crate) fn get(env: &Env) -> Result<&'static TypeConstructors> {
        env.get_instance_data::<TypeConstructors>()?
            .map(|ctors| &*ctors)
            .ok_or_else(|| Error::from_reason("type constructors are not initialized"))
    }
}

fn constructor<Args: JsValuesTupleIntoVec>(
    ctors: &Object,
    name: &str,
) -> Result<FunctionRef<Args, Unknown<'static>>> {
    ctors
        .get_named_property::<Function<Args, Unknown>>(name)
        .map_err(|e| Error::from_reason(format!("type constructor {name}: {e}")))?
        .create_ref()
}

#[napi(
    ts_args_type = "ctors: { SnowflakeDate: new (epochMillis: number, nanos: number, scale: number, timezone: string | number, format: string) => unknown }"
)]
pub fn init_type_constructors(env: Env, ctors: Object) -> Result<()> {
    let ctors = TypeConstructors {
        snowflake_date: constructor(&ctors, "SnowflakeDate")?,
    };
    env.set_instance_data(ctors, (), |_| {})
}
