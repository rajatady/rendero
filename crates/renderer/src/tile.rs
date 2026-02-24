//! Tile management — divides the viewport into cache-friendly tiles.
//!
//! PERFORMANCE: Each tile is 64x64 pixels = 16KB (4 bytes per pixel).
//! This fits in L1 cache on most CPUs.
//! Tiles are independent — can be rendered in parallel with zero contention.

use glam::Vec2;

use crate::scene::{AABB, RenderItem};

/// Tile size in pixels. 64x64 = 16KB per tile (RGBA u8).
pub const TILE_SIZE: u32 = 64;

/// A tile's pixel buffer. RGBA, premultiplied alpha.
#[derive(Clone)]
pub struct TileBuffer {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl TileBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![0u8; (width * height * 4) as usize],
            width,
            height,
        }
    }

    /// Clear to transparent.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Set a pixel. Does bounds checking — no panics on out-of-bounds.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            self.pixels[idx] = r;
            self.pixels[idx + 1] = g;
            self.pixels[idx + 2] = b;
            self.pixels[idx + 3] = a;
        }
    }

    /// Get a pixel.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            (
                self.pixels[idx],
                self.pixels[idx + 1],
                self.pixels[idx + 2],
                self.pixels[idx + 3],
            )
        } else {
            (0, 0, 0, 0)
        }
    }

    /// Blend a premultiplied color onto a pixel (source-over compositing).
    #[inline]
    pub fn blend_pixel(&mut self, x: u32, y: u32, sr: u8, sg: u8, sb: u8, sa: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;

        // Source-over: out = src + dst * (1 - src_alpha)
        let sa_f = sa as f32 / 255.0;
        let inv_sa = 1.0 - sa_f;

        self.pixels[idx] = (sr as f32 + self.pixels[idx] as f32 * inv_sa) as u8;
        self.pixels[idx + 1] = (sg as f32 + self.pixels[idx + 1] as f32 * inv_sa) as u8;
        self.pixels[idx + 2] = (sb as f32 + self.pixels[idx + 2] as f32 * inv_sa) as u8;
        self.pixels[idx + 3] = (sa as f32 + self.pixels[idx + 3] as f32 * inv_sa) as u8;
    }
}

/// A tile's position in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub col: u32,
    pub row: u32,
}

impl TileCoord {
    /// World-space bounding box of this tile.
    pub fn bounds(&self) -> AABB {
        let x = self.col as f32 * TILE_SIZE as f32;
        let y = self.row as f32 * TILE_SIZE as f32;
        AABB::new(
            Vec2::new(x, y),
            Vec2::new(x + TILE_SIZE as f32, y + TILE_SIZE as f32),
        )
    }
}

/// The tile grid covering a viewport.
pub struct TileGrid {
    pub cols: u32,
    pub rows: u32,
    pub viewport: AABB,
}

impl TileGrid {
    /// Create a tile grid covering the given viewport.
    pub fn new(viewport: AABB) -> Self {
        let cols = ((viewport.width()) / TILE_SIZE as f32).ceil() as u32 + 1;
        let rows = ((viewport.height()) / TILE_SIZE as f32).ceil() as u32 + 1;
        Self { cols, rows, viewport }
    }

    /// Get all tiles that intersect with a render item's bounding box.
    pub fn tiles_for_item(&self, item: &RenderItem) -> Vec<TileCoord> {
        let bounds = &item.world_bounds;
        let min_col = ((bounds.min.x / TILE_SIZE as f32).floor() as i32).max(0) as u32;
        let min_row = ((bounds.min.y / TILE_SIZE as f32).floor() as i32).max(0) as u32;
        let max_col = ((bounds.max.x / TILE_SIZE as f32).ceil() as u32).min(self.cols);
        let max_row = ((bounds.max.y / TILE_SIZE as f32).ceil() as u32).min(self.rows);

        let mut tiles = Vec::new();
        for row in min_row..max_row {
            for col in min_col..max_col {
                tiles.push(TileCoord { col, row });
            }
        }
        tiles
    }

    /// Total number of tiles.
    pub fn total_tiles(&self) -> u32 {
        self.cols * self.rows
    }

    /// Iterate all tile coordinates.
    pub fn all_tiles(&self) -> impl Iterator<Item = TileCoord> {
        let cols = self.cols;
        let rows = self.rows;
        (0..rows).flat_map(move |row| (0..cols).map(move |col| TileCoord { col, row }))
    }
}
