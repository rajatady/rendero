//! Essential transformations to make raw kiwi JSON compatible with fig_import.rs expectations.
//! Only the transformations needed by our Node converter — not all 62 from fig2json.

use serde_json::Value as JsonValue;

/// Apply all essential transformations to the document tree.
pub fn apply_essential_transforms(doc: &mut JsonValue) {
    transform_recursive(doc);
}

fn transform_recursive(value: &mut JsonValue) {
    match value {
        JsonValue::Object(map) => {
            // Transform color objects to hex strings
            transform_colors(map);

            // Transform 2x3 matrix to CSS properties
            transform_matrix(map);

            // Transform image hash arrays to filename strings
            transform_image_hash(map);

            // Recurse into all values
            for val in map.values_mut() {
                transform_recursive(val);
            }
        }
        JsonValue::Array(arr) => {
            for val in arr.iter_mut() {
                transform_recursive(val);
            }
        }
        _ => {}
    }
}

/// Convert color: {r, g, b, a} to color: "#rrggbbaa"
fn transform_colors(map: &mut serde_json::Map<String, JsonValue>) {
    let color_fields: Vec<String> = map
        .iter()
        .filter_map(|(k, v)| {
            if k == "color" && v.is_object() {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect();

    for key in color_fields {
        if let Some(color_obj) = map.get(&key).and_then(|v| v.as_object()) {
            let r = color_obj.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let g = color_obj.get("g").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = color_obj.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let a = color_obj.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0);

            let ri = (r * 255.0).round() as u8;
            let gi = (g * 255.0).round() as u8;
            let bi = (b * 255.0).round() as u8;
            let ai = (a * 255.0).round() as u8;

            let hex = if ai == 255 {
                format!("#{:02x}{:02x}{:02x}", ri, gi, bi)
            } else {
                format!("#{:02x}{:02x}{:02x}{:02x}", ri, gi, bi, ai)
            };
            map.insert(key, JsonValue::String(hex));
        }
    }
}

/// Convert transform: {m00, m01, m02, m10, m11, m12} to {x, y, rotation, scaleX, scaleY}
fn transform_matrix(map: &mut serde_json::Map<String, JsonValue>) {
    if let Some(t) = map.get("transform").and_then(|v| v.as_object()) {
        // Check if it's already in CSS format (has "x" field)
        if t.contains_key("x") {
            return;
        }

        // Matrix format: m00, m01, m02 (first row), m10, m11, m12 (second row)
        let m00 = t.get("m00").and_then(|v| v.as_f64());
        let m01 = t.get("m01").and_then(|v| v.as_f64());
        let m02 = t.get("m02").and_then(|v| v.as_f64());
        let m10 = t.get("m10").and_then(|v| v.as_f64());
        let m11 = t.get("m11").and_then(|v| v.as_f64());
        let m12 = t.get("m12").and_then(|v| v.as_f64());

        if let (Some(m00), Some(m01), Some(m02), Some(m10), Some(m11), Some(m12)) =
            (m00, m01, m02, m10, m11, m12)
        {
            let x = m02;
            let y = m12;

            // Extract rotation from the matrix
            let rotation = m10.atan2(m00).to_degrees();

            // Extract scale
            let scale_x = (m00 * m00 + m10 * m10).sqrt();
            let scale_y = (m01 * m01 + m11 * m11).sqrt();

            // Extract skew: shear angle from the matrix
            let skew_x = if scale_x.abs() > 1e-6 {
                (m01 * m00 + m11 * m10).atan2(scale_x * scale_x).to_degrees()
            } else {
                0.0
            };

            let mut css = serde_json::Map::new();
            css.insert("x".into(), json_f64(x));
            css.insert("y".into(), json_f64(y));
            if rotation.abs() > 1e-6 {
                css.insert("rotation".into(), json_f64(rotation));
            }
            if (scale_x - 1.0).abs() > 1e-6 {
                css.insert("scaleX".into(), json_f64(scale_x));
            }
            if (scale_y - 1.0).abs() > 1e-6 {
                css.insert("scaleY".into(), json_f64(scale_y));
            }
            if skew_x.abs() > 1e-6 {
                css.insert("skewX".into(), json_f64(skew_x));
            }

            map.insert("transform".into(), JsonValue::Object(css));
        }
    }
}

/// Convert image hash arrays to hex filename strings.
/// "hash": [96, 73, ...] → "filename": "images/6049..."
fn transform_image_hash(map: &mut serde_json::Map<String, JsonValue>) {
    if let Some(image_obj) = map.get_mut("image") {
        if let Some(obj) = image_obj.as_object_mut() {
            if let Some(hash_val) = obj.get("hash") {
                if let Some(hash_arr) = hash_val.as_array() {
                    let hex: String = hash_arr
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| format!("{:02x}", n as u8)))
                        .collect();
                    obj.insert("filename".into(), JsonValue::String(format!("images/{}", hex)));
                    obj.remove("hash");
                }
            }
        }
    }
}

fn json_f64(v: f64) -> JsonValue {
    serde_json::Number::from_f64(v)
        .map(JsonValue::Number)
        .unwrap_or(JsonValue::Null)
}
