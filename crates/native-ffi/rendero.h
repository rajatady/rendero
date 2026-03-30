// rendero.h — C-ABI header for Rendero native engine
// Auto-generated from crates/native-ffi/src/lib.rs
// Link with: librendero_native_ffi.dylib or librendero_native_ffi.a

#ifndef RENDERO_H
#define RENDERO_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ─── Lifecycle ───

void* rendero_create(const char* name, uint32_t client_id);
void  rendero_destroy(void* engine);

// ─── Viewport & Camera ───

void rendero_set_viewport(void* engine, uint32_t w, uint32_t h);
void rendero_set_camera(void* engine, float x, float y, float zoom);
void rendero_get_camera(void* engine, float* out);  // out must hold 3 floats [x, y, zoom]

// ─── Insert Parent ───

void rendero_set_insert_parent(void* engine, uint32_t counter, uint32_t client_id);
void rendero_clear_insert_parent(void* engine);

// ─── Node Creation ───
// Returns packed uint64: (counter << 32) | client_id

uint64_t rendero_add_frame(void* engine, const char* name,
                           float x, float y, float w, float h,
                           float r, float g, float b, float a);

uint64_t rendero_add_text(void* engine, const char* name, const char* text,
                          float x, float y, float font_size,
                          float r, float g, float b, float a);

// ─── Node Properties ───

void rendero_set_node_position(void* engine, uint32_t counter, uint32_t client_id, float x, float y);
void rendero_set_node_size(void* engine, uint32_t counter, uint32_t client_id, float w, float h);
void rendero_set_node_fill(void* engine, uint32_t counter, uint32_t client_id, float r, float g, float b, float a);
void rendero_set_node_corner_radius(void* engine, uint32_t counter, uint32_t client_id, float tl, float tr, float br, float bl);
void rendero_set_node_opacity(void* engine, uint32_t counter, uint32_t client_id, float opacity);
void rendero_set_node_text(void* engine, uint32_t counter, uint32_t client_id, const char* text);
void rendero_set_node_font_size(void* engine, uint32_t counter, uint32_t client_id, float size);
void rendero_set_node_font_weight(void* engine, uint32_t counter, uint32_t client_id, uint16_t weight);
void rendero_set_auto_layout(void* engine, uint32_t counter, uint32_t client_id,
                             uint32_t direction, float spacing,
                             float pad_top, float pad_right, float pad_bottom, float pad_left);

// ─── Selection & Deletion ───

void rendero_select_node(void* engine, uint32_t counter, uint32_t client_id);
void rendero_delete_selected(void* engine);

// ─── Queries ───

void rendero_get_node_bounds(void* engine, uint32_t counter, uint32_t client_id,
                             float* out_x, float* out_y, float* out_w, float* out_h);

// ─── Rendering ───
// Renders to raw RGBA pixels. Caller provides buffer of size width * height * 4.

void rendero_render_pixels(void* engine, uint8_t* buffer, uint32_t width, uint32_t height);

#ifdef __cplusplus
}
#endif

#endif // RENDERO_H
