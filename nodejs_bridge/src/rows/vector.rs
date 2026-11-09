use arrow::array::FixedSizeListArray;
use sf_types::{ReadArrowError, ReadArrowType};

pub(super) fn vector_numbers(
    array: &FixedSizeListArray,
    row: usize,
) -> Result<Vec<f64>, ReadArrowError> {
    Ok(
        match sf_types::SnowflakeVector.read_arrow_type(array, row)? {
            sf_types::VectorCell::Int32(values) => values.iter().map(|&v| v as f64).collect(),
            // A direct f32-to-f64 conversion carries the 32-bit rounding error along, handing JS 1.7999999523162842 for a stored 1.8.
            // Reparsing the shortest decimal that identifies the f32 ("1.8") gives the number JS means by 1.8.
            // Zero skips the reparse because ryu-js drops the sign of -0.
            sf_types::VectorCell::Float32(values) => {
                let mut buffer = ryu_js::Buffer::new();
                values
                    .iter()
                    .map(|&value| {
                        if value.is_finite() && value != 0.0 {
                            buffer
                                .format_finite(value)
                                .parse::<f64>()
                                .unwrap_or_else(|_| {
                                    unreachable!("ryu-js output always parses as f64")
                                })
                        } else {
                            f64::from(value)
                        }
                    })
                    .collect()
            }
        },
    )
}

pub(super) fn vector_json(
    array: &FixedSizeListArray,
    row: usize,
) -> Result<String, ReadArrowError> {
    let mut buffer = ryu_js::Buffer::new();
    let mut json = String::from("[");
    for (index, value) in vector_numbers(array, row)?.into_iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(if value.is_finite() {
            buffer.format_finite(value)
        } else {
            "null"
        });
    }
    json.push(']');
    Ok(json)
}
