use napi::bindgen_prelude::{
    Either, FnArgs, Function, FunctionRef, JsObjectValue, JsValuesTupleIntoVec, Object, Unknown,
};
use napi::{Env, Error, Result};
use napi_derive::napi;

pub(crate) type SnowflakeDateArgs = FnArgs<(f64, u32, u32, Either<String, i32>, String)>;

/// Lives in napi's per-env instance data. An addon gets only one such slot,
/// so other per-env state belongs here too.
pub(crate) struct ClassConstructors {
    pub snowflake_date: FunctionRef<SnowflakeDateArgs, Unknown<'static>>,
}

impl ClassConstructors {
    pub(crate) fn get(env: &Env) -> Result<&'static ClassConstructors> {
        env.get_instance_data::<ClassConstructors>()?
            .map(|ctors| &*ctors)
            .ok_or_else(|| Error::from_reason("class constructors are not registered"))
    }
}

fn constructor<Args: JsValuesTupleIntoVec>(
    constructors: &Object,
    name: &str,
) -> Result<FunctionRef<Args, Unknown<'static>>> {
    constructors
        .get_named_property::<Function<Args, Unknown>>(name)
        .map_err(|e| Error::from_reason(format!("class constructor {name}: {e}")))?
        .create_ref()
}

/// Gives the bridge the constructors to JS classes it needs to create.
///
/// `SnowflakeDate` extends `Date`, and napi classes can't extend built-in JS classes,
/// so the class lives in TypeScript and the bridge calls its constructor.
/// The constructors are stored per JS env, because napi references don't work across envs.
#[napi(
    ts_args_type = "constructors: { SnowflakeDate: new (epochMillis: number, nanos: number, scale: number, timezone: string | number, format: string) => unknown }"
)]
pub fn register_class_constructors(env: Env, constructors: Object) -> Result<()> {
    let constructors = ClassConstructors {
        snowflake_date: constructor(&constructors, "SnowflakeDate")?,
    };
    env.set_instance_data(constructors, (), |_| {})
}
