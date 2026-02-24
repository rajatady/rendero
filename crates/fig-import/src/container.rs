//! ZIP container extraction and chunk parsing for .fig files.

use crate::error::{FigError, Result};
use flate2::read::DeflateDecoder;
use std::io::{Cursor, Read};
use zip::ZipArchive;

const FIGMA_MAGIC: &[u8; 8] = b"fig-kiwi";
const FIGJAM_MAGIC: &[u8; 8] = b"fig-jam.";
const ZIP_MAGIC: &[u8; 2] = b"PK";
const MIN_FILE_SIZE: usize = 12;

/// Parsed .fig file with version and raw chunks.
pub struct ParsedFile {
    pub version: u32,
    pub chunks: Vec<Vec<u8>>,
}

/// Images extracted from the ZIP container.
pub struct ExtractedImages {
    /// (filename, bytes) pairs for images in the ZIP.
    pub images: Vec<(String, Vec<u8>)>,
}

/// Extract canvas.fig bytes and images from a .fig file (which may be a ZIP).
/// Returns (canvas_bytes, images).
pub fn extract_fig(bytes: &[u8]) -> Result<(Vec<u8>, ExtractedImages)> {
    if bytes.len() >= 2 && &bytes[0..2] == ZIP_MAGIC {
        extract_from_zip(bytes)
    } else {
        Ok((bytes.to_vec(), ExtractedImages { images: vec![] }))
    }
}

fn extract_from_zip(bytes: &[u8]) -> Result<(Vec<u8>, ExtractedImages)> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut canvas_bytes = None;
    let mut images = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if name == "canvas.fig" {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            canvas_bytes = Some(contents);
        } else if name.starts_with("images/") && name.len() > 7 {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            images.push((name, contents));
        }
    }

    let canvas = canvas_bytes.ok_or(FigError::CanvasNotFoundInZip)?;
    Ok((canvas, ExtractedImages { images }))
}

/// Validate magic header.
pub fn validate_header(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 8 {
        return Err(FigError::FileTooSmall {
            expected: 8,
            actual: bytes.len(),
        });
    }
    let header = &bytes[0..8];
    if header == FIGMA_MAGIC || header == FIGJAM_MAGIC {
        Ok(())
    } else {
        Err(FigError::InvalidMagicHeader(header.to_vec()))
    }
}

/// Extract and decompress chunks from canvas.fig bytes.
pub fn extract_chunks(bytes: &[u8]) -> Result<ParsedFile> {
    if bytes.len() < MIN_FILE_SIZE {
        return Err(FigError::FileTooSmall {
            expected: MIN_FILE_SIZE,
            actual: bytes.len(),
        });
    }

    let version = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let mut chunks = Vec::new();
    let mut offset = 12;

    while offset < bytes.len() {
        if offset + 4 > bytes.len() {
            break;
        }
        let chunk_length = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + chunk_length > bytes.len() {
            return Err(FigError::IncompleteChunk {
                offset: offset - 4,
                expected: chunk_length,
                actual: bytes.len() - offset,
            });
        }

        let chunk_data = &bytes[offset..offset + chunk_length];
        // Decompress each chunk
        let decompressed = decompress_chunk(chunk_data)?;
        chunks.push(decompressed);
        offset += chunk_length;
    }

    if chunks.len() < 2 {
        return Err(FigError::NotEnoughChunks {
            expected: 2,
            actual: chunks.len(),
        });
    }

    Ok(ParsedFile { version, chunks })
}

/// Decompress a chunk using DEFLATE or Zstandard.
/// Skips decompression for PNG/JPEG data.
fn decompress_chunk(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() >= 2 {
        // PNG magic: [137, 80], JPEG magic: [255, 216]
        if (bytes[0] == 137 && bytes[1] == 80) || (bytes[0] == 255 && bytes[1] == 216) {
            return Ok(bytes.to_vec());
        }
    }

    // Try DEFLATE first
    if let Ok(data) = decompress_deflate(bytes) {
        return Ok(data);
    }

    // Fall back to Zstandard (pure Rust implementation for WASM compat)
    let mut decoder = ruzstd::StreamingDecoder::new(bytes)
        .map_err(|e| FigError::ZipError(format!("Zstd init failed: {}", e)))?;
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut out)
        .map_err(|e| FigError::ZipError(format!("Zstd decompress failed: {}", e)))?;
    Ok(out)
}

fn decompress_deflate(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| FigError::ZipError(format!("DEFLATE failed: {}", e)))?;
    Ok(decompressed)
}
