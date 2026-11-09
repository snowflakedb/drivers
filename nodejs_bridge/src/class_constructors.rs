use crate::error::{BridgeError, ToJsError};
use napi::bindgen_prelude::{
    Either, FnArgs, Function, FunctionRef, JsObjectValue, JsValuesTupleIntoVec, Object, Unknown,
};
use napi::{Env, Result};

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
            .ok_or_else(|| {
                BridgeError::Message("class constructors are not registered".to_string())
                    .to_js_error(*env, None)
            })
    }
}

fn constructor<Args: JsValuesTupleIntoVec>(
    env: Env,
    constructors: &Object,
    name: &str,
) -> Result<FunctionRef<Args, Unknown<'static>>> {
    constructors
        .get_named_property::<Function<Args, Unknown>>(name)
        .map_err(|e| {
            BridgeError::Message(format!("class constructor {name}: {e}")).to_js_error(env, None)
        })?
        .create_ref()
}

pub(crate) fn register_class_constructors(env: Env, constructors: Object) -> Result<()> {
    let constructors = ClassConstructors {
        snowflake_date: constructor(env, &constructors, "SnowflakeDate")?,
    };
    env.set_instance_data(constructors, (), |_| {})
}
