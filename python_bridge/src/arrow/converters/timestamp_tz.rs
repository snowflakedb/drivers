use arrow::array::{ArrayRef, StructArray};
use chrono::{DateTime, Datelike, FixedOffset, TimeZone};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sf_types::{SnowflakeTimestampTz, TzInstant};

use super::Column;
use super::decode::{PyMaterializer, TypedColumn};
use crate::arrow::converters::util::downcast_column;
use crate::arrow::plan::SnowflakeFieldType;

pub(super) fn from_column(
    array: &ArrayRef,
    field_type: &SnowflakeFieldType,
    scale: u32,
) -> PyResult<Column> {
    downcast_column::<StructArray>(array, field_type).map(|array| {
        Column::TimestampTz(TypedColumn::new(
            array,
            SnowflakeTimestampTz { scale },
            TimestampTzMaterializer,
        ))
    })
}

pub(crate) struct TimestampTzMaterializer;

impl PyMaterializer<SnowflakeTimestampTz> for TimestampTzMaterializer {
    fn materialize<'py>(&self, py: Python<'py>, value: TzInstant) -> PyResult<Bound<'py, PyAny>> {
        let datetime = datetime_with_stored_offset(value)?;
        let year = datetime.year();
        if !(1..=9999).contains(&year) {
            return Err(PyValueError::new_err(format!(
                "TIMESTAMP_TZ {datetime} is outside the datetime.datetime year range 1..=9999"
            )));
        }
        Ok(datetime.into_pyobject(py)?.into_any())
    }
}

fn datetime_with_stored_offset(value: TzInstant) -> PyResult<DateTime<FixedOffset>> {
    // Snowflake's biased wire field is 0..=2880 (±1440 minutes). chrono
    // FixedOffset and datetime.timezone reject |offset| >= 24h today, so
    // protocol extremes at ±1440 minutes error here.
    let offset = value
        .offset_minutes
        .checked_mul(60)
        .and_then(FixedOffset::east_opt)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "TIMESTAMP_TZ offset {} minutes is out of range",
                value.offset_minutes
            ))
        })?;
    Ok(offset.from_utc_datetime(&value.utc))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, Date32Array, Int32Array, Int64Array, StructArray};
    use arrow::buffer::NullBuffer;
    use arrow::datatypes::{DataType, Field, Schema};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use sf_types::{TZ_OFFSET_BIAS_MINUTES, TzInstant};

    use super::datetime_with_stored_offset;

    use crate::arrow::converters::ConversionContext;
    use crate::arrow::converters::test_util::{assert_py_datetime_tz, assert_py_none};
    use crate::arrow::plan::SnowflakeFieldType;

    fn tz(scale: u32) -> SnowflakeFieldType {
        SnowflakeFieldType::TimestampTz { scale }
    }

    fn biased(offset_minutes: i32) -> i32 {
        offset_minutes + TZ_OFFSET_BIAS_MINUTES
    }

    fn two_col(epochs: Vec<Option<i64>>, offsets: Vec<Option<i32>>) -> ArrayRef {
        let nulls: Vec<bool> = epochs.iter().map(Option::is_some).collect();
        Arc::new(
            StructArray::try_new(
                vec![
                    Field::new("epoch", DataType::Int64, true),
                    Field::new("timezone", DataType::Int32, true),
                ]
                .into(),
                vec![
                    Arc::new(Int64Array::from(epochs)),
                    Arc::new(Int32Array::from(offsets)),
                ],
                Some(NullBuffer::from(nulls)),
            )
            .unwrap(),
        )
    }

    fn three_col(
        epochs: Vec<Option<i64>>,
        fractions: Vec<Option<i32>>,
        offsets: Vec<Option<i32>>,
    ) -> ArrayRef {
        let nulls: Vec<bool> = epochs.iter().map(Option::is_some).collect();
        Arc::new(
            StructArray::try_new(
                vec![
                    Field::new("epoch", DataType::Int64, true),
                    Field::new("fraction", DataType::Int32, true),
                    Field::new("timezone", DataType::Int32, true),
                ]
                .into(),
                vec![
                    Arc::new(Int64Array::from(epochs)),
                    Arc::new(Int32Array::from(fractions)),
                    Arc::new(Int32Array::from(offsets)),
                ],
                Some(NullBuffer::from(nulls)),
            )
            .unwrap(),
        )
    }

    #[test]
    fn two_and_three_field_structs_convert_to_aware_python_datetime_with_nulls() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();

        let two_field = ctx
            .converter_from_column(
                &two_col(
                    vec![Some(1_705_296_600_123), None],
                    vec![Some(biased(300)), None],
                ),
                &tz(3),
            )
            .unwrap();
        let three_field = ctx
            .converter_from_column(
                &three_col(
                    vec![Some(1_705_296_600), None],
                    vec![Some(123_456_789), None],
                    vec![Some(biased(300)), None],
                ),
                &tz(9),
            )
            .unwrap();

        Python::attach(|py| {
            assert_py_datetime_tz(
                &two_field.to_py(py, 0).unwrap(),
                (2024, 1, 15),
                (10, 30, 0, 123_000),
                300,
            );
            assert_py_none(&two_field.to_py(py, 1).unwrap());

            assert_py_datetime_tz(
                &three_field.to_py(py, 0).unwrap(),
                (2024, 1, 15),
                (10, 30, 0, 123_456),
                300,
            );
            assert_py_none(&three_field.to_py(py, 1).unwrap());
        });
    }

    #[test]
    fn preserves_utc_and_negative_offsets() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = three_col(
            vec![Some(0), Some(1_705_296_600)],
            vec![Some(0), Some(0)],
            vec![Some(biased(0)), Some(biased(-480))],
        );
        let column = ctx.converter_from_column(&array, &tz(9)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_tz(&column.to_py(py, 0).unwrap(), (1970, 1, 1), (0, 0, 0, 0), 0);
            assert_py_datetime_tz(
                &column.to_py(py, 1).unwrap(),
                (2024, 1, 14),
                (21, 30, 0, 0),
                -480,
            );
        });
    }

    #[test]
    fn converts_negative_epoch_with_nonzero_offset() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(
            vec![Some(-86_400), Some(-86_400), Some(-3_600)],
            vec![Some(biased(300)), Some(biased(-480)), Some(biased(60))],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_tz(
                &column.to_py(py, 0).unwrap(),
                (1969, 12, 31),
                (5, 0, 0, 0),
                300,
            );
            assert_py_datetime_tz(
                &column.to_py(py, 1).unwrap(),
                (1969, 12, 30),
                (16, 0, 0, 0),
                -480,
            );
            assert_py_datetime_tz(
                &column.to_py(py, 2).unwrap(),
                (1970, 1, 1),
                (0, 0, 0, 0),
                60,
            );
        });
    }

    #[test]
    fn converts_offsets_just_inside_plus_and_minus_1440() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let just_inside = TZ_OFFSET_BIAS_MINUTES - 1;
        let array = two_col(
            vec![Some(0), Some(0)],
            vec![Some(biased(just_inside)), Some(biased(-just_inside))],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_tz(
                &column.to_py(py, 0).unwrap(),
                (1970, 1, 1),
                (23, 59, 0, 0),
                just_inside,
            );
            assert_py_datetime_tz(
                &column.to_py(py, 1).unwrap(),
                (1969, 12, 31),
                (0, 1, 0, 0),
                -just_inside,
            );
        });
    }

    #[test]
    fn converts_python_datetime_year_bounds() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(
            vec![Some(-62_135_596_800), Some(253_402_300_799)],
            vec![Some(biased(0)), Some(biased(0))],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            assert_py_datetime_tz(&column.to_py(py, 0).unwrap(), (1, 1, 1), (0, 0, 0, 0), 0);
            assert_py_datetime_tz(
                &column.to_py(py, 1).unwrap(),
                (9999, 12, 31),
                (23, 59, 59, 0),
                0,
            );
        });
    }

    #[test]
    fn rejects_physical_mismatch_for_timestamp_tz() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let cases: Vec<ArrayRef> = vec![
            Arc::new(Int64Array::from(vec![Some(0)])),
            Arc::new(Date32Array::from(vec![Some(0)])),
        ];
        for array in cases {
            let err = match ctx.converter_from_column(&array, &tz(0)) {
                Ok(_) => panic!(
                    "expected physical mismatch for TIMESTAMP_TZ, got {:?}",
                    array.data_type()
                ),
                Err(err) => err,
            };
            Python::attach(|py| {
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("logical/physical type mismatch")
                        && text.contains("TIMESTAMP_TZ"),
                    "got {text}"
                );
            });
        }
    }

    #[test]
    fn rejects_datetimes_outside_python_year_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(
            vec![Some(-62_135_683_200), Some(253_402_300_800)],
            vec![Some(biased(0)), Some(biased(0))],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("datetime.datetime year range 1..=9999"),
                    "got {text}"
                );
            }
        });
    }

    #[test]
    fn offset_pushes_datetime_outside_python_year_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(
            vec![Some(-62_135_596_800), Some(253_402_300_799)],
            vec![Some(biased(-1)), Some(biased(1))],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("datetime.datetime year range 1..=9999"),
                    "got {text}"
                );
            }
        });
    }

    #[test]
    fn rejects_raw_values_outside_chrono_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(vec![Some(i64::MAX)], vec![Some(biased(0))]);
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("Invalid Arrow value") && text.contains("timestamp"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_offset_outside_biased_protocol_range() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(vec![Some(0)], vec![Some(TZ_OFFSET_BIAS_MINUTES * 2 + 1)]);
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            let err = column.to_py(py, 0).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("Invalid Arrow value") && text.contains("TIMESTAMP_TZ offset"),
                "got {text}"
            );
        });
    }

    #[test]
    fn rejects_plus_and_minus_1440_minute_offsets() {
        Python::initialize();
        let ctx = ConversionContext::new(&Schema::empty()).unwrap();
        let array = two_col(
            vec![Some(0), Some(0)],
            vec![
                Some(biased(TZ_OFFSET_BIAS_MINUTES)),
                Some(biased(-TZ_OFFSET_BIAS_MINUTES)),
            ],
        );
        let column = ctx.converter_from_column(&array, &tz(0)).unwrap();

        Python::attach(|py| {
            for row in [0, 1] {
                let err = column.to_py(py, row).unwrap_err();
                assert!(
                    err.is_instance_of::<PyValueError>(py),
                    "expected PyValueError, got {err}"
                );
                let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
                assert!(
                    text.contains("TIMESTAMP_TZ offset") && text.contains("out of range"),
                    "got {text}"
                );
            }
        });
    }

    #[test]
    fn rejects_hand_built_offset_outside_fixedoffset_range() {
        Python::initialize();
        let value = TzInstant {
            utc: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            offset_minutes: TZ_OFFSET_BIAS_MINUTES,
        };
        Python::attach(|py| {
            let err = datetime_with_stored_offset(value).unwrap_err();
            assert!(
                err.is_instance_of::<PyValueError>(py),
                "expected PyValueError, got {err}"
            );
            let text = err.value(py).str().unwrap().to_string_lossy().into_owned();
            assert!(
                text.contains("TIMESTAMP_TZ offset") && text.contains("out of range"),
                "got {text}"
            );
        });
    }
}
