//! fig_import: .fig binary → JSON tree with essential transforms.
//!
//! Pipeline:
//! 1. ZIP extraction (if needed) + image extraction
//! 2. Header validation + chunk extraction + decompression
//! 3. Kiwi schema decode → serde_json::Value
//! 4. Flat nodeChanges → hierarchical tree
//! 5. Blob substitution (commandsBlob → commands, vectorNetworkBlob → vectorNetwork)
//! 6. Essential transforms (color→hex, matrix→CSS, image hash→filename)
//!
//! The output JSON is compatible with the existing fig_import.rs converter in crates/wasm.

mod blobs;
mod container;
mod decode;
mod error;
mod transform;
mod tree;

pub use error::{FigError, Result};

/// Result of importing a .fig file.
pub struct FigImportResult {
    /// The document tree as JSON, ready for fig_import.rs conversion.
    pub document: serde_json::Value,
    /// Images extracted from the ZIP: (path, bytes).
    pub images: Vec<(String, Vec<u8>)>,
    /// File format version.
    pub version: u32,
}

/// Convert .fig bytes to a JSON document tree with images.
/// The returned JSON is compatible with the existing fig_import.rs converter.
pub fn convert_fig(bytes: &[u8]) -> Result<FigImportResult> {
    // 1. Extract canvas.fig + images from ZIP (or use raw bytes)
    let (canvas_bytes, extracted_images) = container::extract_fig(bytes)?;

    // 2. Validate header
    container::validate_header(&canvas_bytes)?;

    // 3. Extract and decompress chunks
    let parsed = container::extract_chunks(&canvas_bytes)?;

    // 4. Kiwi decode (schema = chunk 0, data = chunk 1)
    let json = decode::decode_fig_to_json(&parsed.chunks[0], &parsed.chunks[1])?;

    // 5. Extract nodeChanges and build tree
    // Take ownership to avoid cloning (critical for large files — apple.fig is 97MB)
    let mut json_map = match json {
        serde_json::Value::Object(map) => map,
        _ => return Err(FigError::DecodeError("Decoded data is not an object".into())),
    };

    let node_changes = match json_map.remove("nodeChanges") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => return Err(FigError::DecodeError("No nodeChanges in decoded data".into())),
    };

    let mut document = tree::build_tree(node_changes)?;

    // 6. Process and substitute blobs
    let blobs_arr = match json_map.remove("blobs") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => Vec::new(),
    };

    let processed_blobs = blobs::process_blobs(blobs_arr)?;
    blobs::substitute_blobs(&mut document, &processed_blobs)?;

    // 7. Apply essential transforms (color→hex, matrix→CSS, image hash→filename)
    transform::apply_essential_transforms(&mut document);

    Ok(FigImportResult {
        document,
        images: extracted_images.images,
        version: parsed.version,
    })
}
