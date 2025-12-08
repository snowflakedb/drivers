use sf_core::query_types::{QueryTypesError, RowType};

#[test]
fn test_fixed_type_constructor() {
    let fixed = RowType::fixed("col1", false, 10, 0).unwrap();
    assert_eq!(fixed.name(), "col1");
    assert_eq!(fixed.type_name(), "FIXED");
}

#[test]
fn test_decimal_type_constructor() {
    let decimal = RowType::decimal("col1", true, 38, 10).unwrap();
    assert_eq!(decimal.name(), "col1");
    assert_eq!(decimal.type_name(), "DECIMAL");
}

#[test]
fn test_decimal_invalid_scale() {
    let result = RowType::decimal("col1", false, 10, 20);
    assert!(result.is_err());
}

#[test]
fn test_decimal_invalid_precision() {
    let result = RowType::decimal("col1", false, 40, 10);
    assert!(result.is_err());
}

#[test]
fn test_real_type_constructor() {
    let real = RowType::real("col1", false);
    assert_eq!(real.name(), "col1");
    assert_eq!(real.type_name(), "REAL");
}

#[test]
fn test_double_type_constructor() {
    let double = RowType::double("col1", false);
    assert_eq!(double.name(), "col1");
    assert_eq!(double.type_name(), "DOUBLE");
}

#[test]
fn test_boolean_type_constructor() {
    let bool_type = RowType::boolean("col1", true);
    assert_eq!(bool_type.name(), "col1");
    assert_eq!(bool_type.type_name(), "BOOLEAN");
}

#[test]
fn test_date_type_constructor() {
    let date = RowType::date("col1", false);
    assert_eq!(date.name(), "col1");
    assert_eq!(date.type_name(), "DATE");
}

#[test]
fn test_time_type_constructor() {
    let time = RowType::time("col1", false, 6).unwrap();
    assert_eq!(time.name(), "col1");
    assert_eq!(time.type_name(), "TIME");
}

#[test]
fn test_time_invalid_precision() {
    let result = RowType::time("col1", false, 10);
    assert!(result.is_err());
}

#[test]
fn test_timestamp_ntz_constructor() {
    let ts = RowType::timestamp_ntz("col1", false, 9).unwrap();
    assert_eq!(ts.name(), "col1");
    assert_eq!(ts.type_name(), "TIMESTAMP_NTZ");
}

#[test]
fn test_timestamp_ltz_constructor() {
    let ts = RowType::timestamp_ltz("col1", false, 9).unwrap();
    assert_eq!(ts.name(), "col1");
    assert_eq!(ts.type_name(), "TIMESTAMP_LTZ");
}

#[test]
fn test_timestamp_tz_constructor() {
    let ts = RowType::timestamp_tz("col1", false, 9).unwrap();
    assert_eq!(ts.name(), "col1");
    assert_eq!(ts.type_name(), "TIMESTAMP_TZ");
}

#[test]
fn test_variant_type_constructor() {
    let variant = RowType::variant("col1", true);
    assert_eq!(variant.name(), "col1");
    assert_eq!(variant.type_name(), "VARIANT");
}

#[test]
fn test_object_type_constructor() {
    let object = RowType::object("col1", true);
    assert_eq!(object.name(), "col1");
    assert_eq!(object.type_name(), "OBJECT");
}

#[test]
fn test_array_type_constructor() {
    let array = RowType::array("col1", true);
    assert_eq!(array.name(), "col1");
    assert_eq!(array.type_name(), "ARRAY");
}

#[test]
fn test_geography_type_constructor() {
    let geo = RowType::geography("col1", false);
    assert_eq!(geo.name(), "col1");
    assert_eq!(geo.type_name(), "GEOGRAPHY");
}

#[test]
fn test_geometry_type_constructor() {
    let geom = RowType::geometry("col1", false);
    assert_eq!(geom.name(), "col1");
    assert_eq!(geom.type_name(), "GEOMETRY");
}

#[test]
fn test_binary_type_constructor() {
    let binary = RowType::binary("col1", false, 100);
    assert_eq!(binary.name(), "col1");
    assert_eq!(binary.type_name(), "BINARY");
}

#[test]
fn test_varbinary_type_constructor() {
    let varbinary = RowType::varbinary("col1", false, 8388608);
    assert_eq!(varbinary.name(), "col1");
    assert_eq!(varbinary.type_name(), "VARBINARY");
}
