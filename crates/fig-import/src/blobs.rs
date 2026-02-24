//! Blob processing: base64 encoding + substitution + binary parsing.

use crate::error::{FigError, Result};
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value as JsonValue;

/// Process blobs array: encode binary byte arrays to base64 strings.
pub fn process_blobs(blobs: Vec<JsonValue>) -> Result<Vec<JsonValue>> {
    let mut processed = Vec::new();
    for blob in blobs {
        let mut pb = blob.clone();
        if let Some(obj) = pb.as_object_mut() {
            if let Some(bytes_value) = obj.get("bytes") {
                if let Some(bytes_array) = bytes_value.as_array() {
                    let bytes: Vec<u8> = bytes_array
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    obj.insert("bytes".to_string(), JsonValue::String(b64));
                }
            }
        }
        processed.push(pb);
    }
    Ok(processed)
}

/// Substitute blob references in the tree.
/// Replaces fields like "commandsBlob: 5" with "commands: [parsed content]".
pub fn substitute_blobs(tree: &mut JsonValue, blobs: &[JsonValue]) -> Result<()> {
    substitute_recursive(tree, blobs)
}

fn substitute_recursive(value: &mut JsonValue, blobs: &[JsonValue]) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            let mut replacements = Vec::new();
            for (key, val) in map.iter() {
                if key.ends_with("Blob") {
                    if let Some(index) = val.as_u64() {
                        let index = index as usize;
                        if index < blobs.len() {
                            let blob_type = &key[..key.len() - 4];
                            if let Some(parsed) = parse_blob(blob_type, &blobs[index])? {
                                replacements.push((key.clone(), blob_type.to_string(), parsed));
                            }
                        }
                    }
                }
            }
            for (old_key, new_key, new_value) in replacements {
                map.remove(&old_key);
                map.insert(new_key, new_value);
            }
            for val in map.values_mut() {
                substitute_recursive(val, blobs)?;
            }
        }
        JsonValue::Array(arr) => {
            for val in arr.iter_mut() {
                substitute_recursive(val, blobs)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_blob(blob_type: &str, blob: &JsonValue) -> Result<Option<JsonValue>> {
    let bytes = extract_blob_bytes(blob)?;
    match blob_type {
        "commands" => Ok(parse_commands(&bytes)),
        "vectorNetwork" => Ok(parse_vector_network(&bytes)),
        _ => Ok(None),
    }
}

fn extract_blob_bytes(blob: &JsonValue) -> Result<Vec<u8>> {
    let bytes_value = blob
        .get("bytes")
        .ok_or_else(|| FigError::DecodeError("Blob missing bytes field".into()))?;

    if let Some(b64) = bytes_value.as_str() {
        return general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| FigError::DecodeError(format!("base64 decode failed: {}", e)));
    }
    if let Some(arr) = bytes_value.as_array() {
        return Ok(arr
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect());
    }
    Err(FigError::DecodeError("Blob bytes: not string or array".into()))
}

fn parse_commands(bytes: &[u8]) -> Option<JsonValue> {
    let mut commands = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let cmd = bytes[offset];
        offset += 1;
        match cmd {
            0 => commands.push(JsonValue::String("Z".into())),
            1 => {
                if offset + 8 > bytes.len() { return None; }
                let x = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
                let y = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
                offset += 8;
                commands.push(JsonValue::String("M".into()));
                commands.push(json_f32(x));
                commands.push(json_f32(y));
            }
            2 => {
                if offset + 8 > bytes.len() { return None; }
                let x = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
                let y = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
                offset += 8;
                commands.push(JsonValue::String("L".into()));
                commands.push(json_f32(x));
                commands.push(json_f32(y));
            }
            3 => {
                if offset + 16 > bytes.len() { return None; }
                let cx = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
                let cy = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
                let x = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]);
                let y = f32::from_le_bytes([bytes[offset+12], bytes[offset+13], bytes[offset+14], bytes[offset+15]]);
                offset += 16;
                commands.push(JsonValue::String("Q".into()));
                commands.push(json_f32(cx));
                commands.push(json_f32(cy));
                commands.push(json_f32(x));
                commands.push(json_f32(y));
            }
            4 => {
                if offset + 24 > bytes.len() { return None; }
                let cx1 = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
                let cy1 = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
                let cx2 = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]);
                let cy2 = f32::from_le_bytes([bytes[offset+12], bytes[offset+13], bytes[offset+14], bytes[offset+15]]);
                let x = f32::from_le_bytes([bytes[offset+16], bytes[offset+17], bytes[offset+18], bytes[offset+19]]);
                let y = f32::from_le_bytes([bytes[offset+20], bytes[offset+21], bytes[offset+22], bytes[offset+23]]);
                offset += 24;
                commands.push(JsonValue::String("C".into()));
                commands.push(json_f32(cx1));
                commands.push(json_f32(cy1));
                commands.push(json_f32(cx2));
                commands.push(json_f32(cy2));
                commands.push(json_f32(x));
                commands.push(json_f32(y));
            }
            _ => return None,
        }
    }
    Some(JsonValue::Array(commands))
}

fn parse_vector_network(bytes: &[u8]) -> Option<JsonValue> {
    if bytes.len() < 12 { return None; }

    let vertex_count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let segment_count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let region_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let mut offset = 12;

    let mut vertices = Vec::new();
    for _ in 0..vertex_count {
        if offset + 12 > bytes.len() { return None; }
        let style_id = u32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
        let x = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
        let y = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]);
        offset += 12;
        vertices.push(serde_json::json!({"styleID": style_id, "x": json_f32(x), "y": json_f32(y)}));
    }

    let mut segments = Vec::new();
    for _ in 0..segment_count {
        if offset + 28 > bytes.len() { return None; }
        let style_id = u32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
        let sv = u32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]);
        let sdx = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]);
        let sdy = f32::from_le_bytes([bytes[offset+12], bytes[offset+13], bytes[offset+14], bytes[offset+15]]);
        let ev = u32::from_le_bytes([bytes[offset+16], bytes[offset+17], bytes[offset+18], bytes[offset+19]]);
        let edx = f32::from_le_bytes([bytes[offset+20], bytes[offset+21], bytes[offset+22], bytes[offset+23]]);
        let edy = f32::from_le_bytes([bytes[offset+24], bytes[offset+25], bytes[offset+26], bytes[offset+27]]);
        offset += 28;
        if sv as usize >= vertex_count || ev as usize >= vertex_count { return None; }
        segments.push(serde_json::json!({
            "styleID": style_id,
            "start": {"vertex": sv, "dx": json_f32(sdx), "dy": json_f32(sdy)},
            "end": {"vertex": ev, "dx": json_f32(edx), "dy": json_f32(edy)}
        }));
    }

    let mut regions = Vec::new();
    for _ in 0..region_count {
        if offset + 8 > bytes.len() { return None; }
        let style_and_rule = u32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
        let winding = if style_and_rule & 1 != 0 { "NONZERO" } else { "ODD" };
        let style_id = style_and_rule >> 1;
        let loop_count = u32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]) as usize;
        offset += 8;

        let mut loops = Vec::new();
        for _ in 0..loop_count {
            if offset + 4 > bytes.len() { return None; }
            let idx_count = u32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]) as usize;
            offset += 4;
            if offset + idx_count * 4 > bytes.len() { return None; }
            let mut indices = Vec::new();
            for _ in 0..idx_count {
                let si = u32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]);
                offset += 4;
                if si as usize >= segment_count { return None; }
                indices.push(JsonValue::Number(si.into()));
            }
            loops.push(serde_json::json!({"segments": indices}));
        }
        regions.push(serde_json::json!({"styleID": style_id, "windingRule": winding, "loops": loops}));
    }

    Some(serde_json::json!({"vertices": vertices, "segments": segments, "regions": regions}))
}

fn json_f32(v: f32) -> JsonValue {
    if v.is_nan() || v.is_infinite() {
        JsonValue::Null
    } else {
        serde_json::Number::from_f64(v as f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null)
    }
}
