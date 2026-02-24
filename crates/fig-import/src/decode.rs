//! Kiwi schema decoding: binary → serde_json::Value tree.

use crate::error::{FigError, Result};
use kiwi_schema::{Schema, Value};
use serde_json::Value as JsonValue;

/// Decode .fig schema+data chunks into a JSON Value.
/// Returns the root "Message" object with nodeChanges and blobs.
pub fn decode_fig_to_json(schema_bytes: &[u8], data_bytes: &[u8]) -> Result<JsonValue> {
    let schema = Schema::decode(schema_bytes)
        .map_err(|_| FigError::DecodeError("Failed to decode Kiwi binary schema".into()))?;

    // Find the root "Message" type with nodeChanges+blobs fields
    let root_type_id = schema
        .defs
        .iter()
        .find(|def| {
            def.name == "Message"
                && def.fields.iter().any(|f| f.name == "nodeChanges")
                && def.fields.iter().any(|f| f.name == "blobs")
        })
        .map(|def| def.index)
        .ok_or_else(|| FigError::DecodeError("No root Message definition in schema".into()))?;

    let value = Value::decode(&schema, root_type_id, data_bytes)
        .map_err(|_| FigError::DecodeError("Failed to decode message data".into()))?;

    Ok(kiwi_to_json(&value))
}

fn kiwi_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Byte(n) => JsonValue::Number((*n).into()),
        Value::Int(n) => JsonValue::Number((*n).into()),
        Value::UInt(n) => JsonValue::Number((*n).into()),
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() {
                JsonValue::Null
            } else {
                serde_json::Number::from_f64(*f as f64)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            }
        }
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Int64(n) => JsonValue::Number((*n).into()),
        Value::UInt64(n) => JsonValue::Number((*n).into()),
        Value::Array(arr) => JsonValue::Array(arr.iter().map(kiwi_to_json).collect()),
        Value::Enum(_enum_name, variant_name) => {
            JsonValue::String(variant_name.to_string())
        }
        Value::Object(_type_name, fields) => {
            let mut map = serde_json::Map::new();
            for (field_name, field_value) in fields {
                map.insert(field_name.to_string(), kiwi_to_json(field_value));
            }
            JsonValue::Object(map)
        }
    }
}
