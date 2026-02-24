//! Render pipeline — the complete Document → Pixels path.
//!
//! TYPE-ENFORCED PIPELINE:
//!   DocumentTree → [build_scene] → Vec<RenderItem> → [render_tiles] → Vec<TileBuffer>
//!
//! Each stage's output type is the next stage's input type.
//! The compiler prevents skipping stages or feeding wrong data.

use std::collections::HashMap;

use rendero_core::id::NodeId;
use rendero_core::tree::DocumentTree;

use rendero_core::properties::Effect;

use crate::scene::{self, AABB, RenderItem};
use crate::tile::{TileBuffer, TileCoord, TileGrid, TILE_SIZE};
use crate::rasterize;

/// Compute expanded bounds that include drop shadow extents.
fn shadow_expanded_bounds(item: &RenderItem) -> Option<AABB> {
    let mut max_expand = 0.0f32;
    let mut has_shadow = false;
    for effect in &item.style.effects {
        if let Effect::DropShadow { offset, blur_radius, spread, .. } = effect {
            let expand = offset.x.abs().max(offset.y.abs()) + blur_radius + spread;
            max_expand = max_expand.max(expand);
            has_shadow = true;
        }
    }
    if !has_shadow { return None; }
    Some(AABB::new(
        glam::Vec2::new(item.world_bounds.min.x - max_expand, item.world_bounds.min.y - max_expand),
        glam::Vec2::new(item.world_bounds.max.x + max_expand, item.world_bounds.max.y + max_expand),
    ))
}

/// The complete render output.
pub struct RenderOutput {
    pub tiles: HashMap<TileCoord, TileBuffer>,
    pub grid: TileGrid,
    pub item_count: usize,
}

impl RenderOutput {
    /// Assemble tiles into a single pixel buffer.
    pub fn to_pixels(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let stride = (width * 4) as usize;

        for (coord, tile) in &self.tiles {
            let base_x = coord.col * TILE_SIZE;
            let base_y = coord.row * TILE_SIZE;
            let tile_w = tile.width.min(TILE_SIZE).min(width.saturating_sub(base_x));
            let tile_h = tile.height.min(TILE_SIZE).min(height.saturating_sub(base_y));
            if tile_w == 0 || tile_h == 0 { continue; }

            let row_bytes = (tile_w * 4) as usize;
            let tile_stride = (tile.width * 4) as usize;

            for y in 0..tile_h {
                let dst_off = ((base_y + y) as usize) * stride + (base_x as usize) * 4;
                let src_off = (y as usize) * tile_stride;
                pixels[dst_off..dst_off + row_bytes]
                    .copy_from_slice(&tile.pixels[src_off..src_off + row_bytes]);
            }
        }

        pixels
    }
}

/// Render a document tree to pixels.
///
/// This is the single entry point for rendering.
/// Pipeline: Tree → Scene → Tiles → Pixels
pub fn render(tree: &DocumentTree, root: &NodeId, viewport: AABB) -> RenderOutput {
    let items = scene::build_scene(tree, root, &viewport);
    render_items(&items, viewport)
}

/// Render pre-built scene items (with camera transforms already applied).
pub fn render_items(items: &[RenderItem], viewport: AABB) -> RenderOutput {
    let grid = TileGrid::new(viewport);
    let mut tiles: HashMap<TileCoord, TileBuffer> = HashMap::new();

    // Clip stack: (end_index, clip_bounds)
    let mut clip_stack: Vec<(usize, AABB)> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        // Pop expired clips
        while let Some((end, _)) = clip_stack.last() {
            if i >= *end {
                clip_stack.pop();
            } else {
                break;
            }
        }

        // Compute effective clip bounds
        let clip_bounds = clip_stack.last().map(|(_, b)| *b);

        // Rasterize with optional clipping
        if let Some(clip) = clip_bounds {
            let clipped_item = RenderItem {
                world_bounds: item.world_bounds.intersect(&clip),
                ..item.clone()
            };
            if clipped_item.world_bounds.min.x < clipped_item.world_bounds.max.x
                && clipped_item.world_bounds.min.y < clipped_item.world_bounds.max.y
            {
                rasterize_with_effects(&clipped_item, &grid, &mut tiles, Some(&clip));
            }
        } else {
            rasterize_with_effects(item, &grid, &mut tiles, None);
        }

        // Push clip if this item clips children
        if item.clips && item.descendant_count > 0 {
            let clip = if let Some(parent_clip) = clip_bounds {
                item.world_bounds.intersect(&parent_clip)
            } else {
                item.world_bounds
            };
            clip_stack.push((i + 1 + item.descendant_count, clip));
        }
    }

    RenderOutput {
        item_count: items.len(),
        tiles,
        grid,
    }
}

/// Rasterize an item with its effects (shadows first, then the item).
fn rasterize_with_effects(
    item: &RenderItem,
    grid: &TileGrid,
    tiles: &mut HashMap<TileCoord, TileBuffer>,
    clip: Option<&AABB>,
) {
    let has_shadows = item.style.effects.iter().any(|e| matches!(e, Effect::DropShadow { .. }));

    if has_shadows {
        // Shadow tiles may extend beyond the item bounds
        let shadow_bounds = shadow_expanded_bounds(item);
        let shadow_item = if let Some(bounds) = shadow_bounds {
            // Create a temporary RenderItem with expanded bounds for tile lookup
            RenderItem {
                world_bounds: bounds,
                ..item.clone()
            }
        } else {
            item.clone()
        };

        let affected_tiles = grid.tiles_for_item(&shadow_item);
        for coord in &affected_tiles {
            let tile = tiles
                .entry(*coord)
                .or_insert_with(|| TileBuffer::new(TILE_SIZE, TILE_SIZE));
            rasterize::rasterize_drop_shadows(
                tile, coord, &item.shape, &item.style.effects, &item.world_transform,
            );
        }
    }

    // Rasterize the item itself
    let affected_tiles = grid.tiles_for_item(item);
    for coord in affected_tiles {
        let tile = tiles
            .entry(coord)
            .or_insert_with(|| TileBuffer::new(TILE_SIZE, TILE_SIZE));

        // Save tile state before rasterizing if we need to clip
        let saved = if clip.is_some() { Some(tile.clone()) } else { None };

        rasterize::rasterize_item(
            tile,
            &coord,
            &item.shape,
            &item.style.fills,
            item.style.opacity,
            &item.world_transform,
        );

        // Apply clip mask: restore pixels outside clip bounds
        if let (Some(clip_bounds), Some(saved_tile)) = (clip, &saved) {
            let tile_x = coord.col * TILE_SIZE;
            let tile_y = coord.row * TILE_SIZE;
            for py in 0..TILE_SIZE {
                for px in 0..TILE_SIZE {
                    let wx = (tile_x + px) as f32 + 0.5;
                    let wy = (tile_y + py) as f32 + 0.5;
                    if wx < clip_bounds.min.x || wx > clip_bounds.max.x
                        || wy < clip_bounds.min.y || wy > clip_bounds.max.y
                    {
                        // Restore pixel from saved tile
                        let (r, g, b, a) = saved_tile.get_pixel(px, py);
                        tile.set_pixel(px, py, r, g, b, a);
                    }
                }
            }
        }
    }
}
