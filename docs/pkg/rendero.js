/* @ts-self-types="./rendero.d.ts" */

export class CanvasEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CanvasEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_canvasengine_free(ptr, 0);
    }
    /**
     * Add a layer blur effect to a node.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} radius
     * @returns {boolean}
     */
    add_blur(counter, client_id, radius) {
        const ret = wasm.canvasengine_add_blur(this.__wbg_ptr, counter, client_id, radius);
        return ret !== 0;
    }
    /**
     * Add a comment at world position (x, y). Returns the comment ID.
     * @param {number} x
     * @param {number} y
     * @param {string} text
     * @param {string} author
     * @returns {number}
     */
    add_comment(x, y, text, author) {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_comment(this.__wbg_ptr, x, y, ptr0, len0, ptr1, len1);
        return ret >>> 0;
    }
    /**
     * Add a drop shadow effect to a node.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} ox
     * @param {number} oy
     * @param {number} blur
     * @param {number} spread
     * @returns {boolean}
     */
    add_drop_shadow(counter, client_id, r, g, b, a, ox, oy, blur, spread) {
        const ret = wasm.canvasengine_add_drop_shadow(this.__wbg_ptr, counter, client_id, r, g, b, a, ox, oy, blur, spread);
        return ret !== 0;
    }
    /**
     * Add an ellipse. Returns node ID as [counter, client_id].
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {Uint32Array}
     */
    add_ellipse(name, x, y, width, height, r, g, b, a) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_ellipse(this.__wbg_ptr, ptr0, len0, x, y, width, height, r, g, b, a);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Batch add multiple ellipses in one call. Format: [x, y, w, h, r, g, b, a] × N.
     * Skips CRDT ops, undo stack, and per-node scene updates for maximum throughput.
     * Returns the number of ellipses added.
     * @param {Float32Array} data
     * @returns {number}
     */
    add_ellipses_batch(data) {
        const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_ellipses_batch(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Add a frame.
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} w
     * @param {number} h
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {Uint32Array}
     */
    add_frame(name, x, y, w, h, r, g, b, a) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_frame(this.__wbg_ptr, ptr0, len0, x, y, w, h, r, g, b, a);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Add a rectangle with a linear gradient fill.
     * stop_positions and stop_colors are parallel arrays. Each color is [r, g, b, a].
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} start_x
     * @param {number} start_y
     * @param {number} end_x
     * @param {number} end_y
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {Uint32Array}
     */
    add_gradient_rectangle(name, x, y, width, height, start_x, start_y, end_x, end_y, stop_positions, stop_colors) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_gradient_rectangle(this.__wbg_ptr, ptr0, len0, x, y, width, height, start_x, start_y, end_x, end_y, ptr1, len1, ptr2, len2);
        var v4 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v4;
    }
    /**
     * Add an image node from raw RGBA pixel data.
     * Returns node ID as [counter, client_id].
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {Uint8Array} image_data
     * @param {number} image_width
     * @param {number} image_height
     * @returns {Uint32Array}
     */
    add_image(name, x, y, width, height, image_data, image_width, image_height) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(image_data, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_image(this.__wbg_ptr, ptr0, len0, x, y, width, height, ptr1, len1, image_width, image_height);
        var v3 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v3;
    }
    /**
     * Add a rectangle with an image fill (URL-based, loaded by renderer).
     * `path` is relative to /imports/ (e.g. "starbucks.png").
     * `scale_mode`: "fill", "fit", "tile", "stretch".
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {string} path
     * @param {string} scale_mode
     * @param {number} opacity
     * @returns {Uint32Array}
     */
    add_image_fill(name, x, y, width, height, path, scale_mode, opacity) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(scale_mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_image_fill(this.__wbg_ptr, ptr0, len0, x, y, width, height, ptr1, len1, ptr2, len2, opacity);
        var v4 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v4;
    }
    /**
     * Add an inner shadow effect to a node.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} ox
     * @param {number} oy
     * @param {number} blur
     * @param {number} spread
     * @returns {boolean}
     */
    add_inner_shadow(counter, client_id, r, g, b, a, ox, oy, blur, spread) {
        const ret = wasm.canvasengine_add_inner_shadow(this.__wbg_ptr, counter, client_id, r, g, b, a, ox, oy, blur, spread);
        return ret !== 0;
    }
    /**
     * Add a line from (x1,y1) to (x2,y2) with stroke color.
     * @param {string} name
     * @param {number} x1
     * @param {number} y1
     * @param {number} x2
     * @param {number} y2
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} stroke_width
     * @returns {Uint32Array}
     */
    add_line(name, x1, y1, x2, y2, r, g, b, a, stroke_width) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_line(this.__wbg_ptr, ptr0, len0, x1, y1, x2, y2, r, g, b, a, stroke_width);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Append a solid fill to existing fills (for multiple fills per node).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {boolean}
     */
    add_node_fill(counter, client_id, r, g, b, a) {
        const ret = wasm.canvasengine_add_node_fill(this.__wbg_ptr, counter, client_id, r, g, b, a);
        return ret !== 0;
    }
    /**
     * Append a linear gradient fill to existing fills.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} start_x
     * @param {number} start_y
     * @param {number} end_x
     * @param {number} end_y
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    add_node_linear_gradient(counter, client_id, start_x, start_y, end_x, end_y, stop_positions, stop_colors) {
        const ptr0 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_node_linear_gradient(this.__wbg_ptr, counter, client_id, start_x, start_y, end_x, end_y, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Append a radial gradient fill to existing fills.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} center_x
     * @param {number} center_y
     * @param {number} radius
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    add_node_radial_gradient(counter, client_id, center_x, center_y, radius, stop_positions, stop_colors) {
        const ptr0 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_node_radial_gradient(this.__wbg_ptr, counter, client_id, center_x, center_y, radius, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Add a new page and return its index.
     * @param {string} name
     * @returns {number}
     */
    add_page(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_page(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Add a GPU-direct point cloud from packed Float32Array: [x, y, w, h, r, g, b, a] × N.
     * Point clouds bypass the document tree entirely — data goes straight to GPU.
     * Returns cloud ID.
     * @param {WebGL2RenderingContext} gl
     * @param {Float32Array} data
     * @returns {number}
     */
    add_point_cloud(gl, data) {
        const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_point_cloud(this.__wbg_ptr, gl, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Add a prototype link from source node to target node.
     * trigger: "click" | "hover" | "drag"
     * animation: "instant" | "dissolve" | "slide"
     * @param {number} src_counter
     * @param {number} src_client
     * @param {number} dst_counter
     * @param {number} dst_client
     * @param {string} trigger
     * @param {string} animation
     * @returns {boolean}
     */
    add_prototype_link(src_counter, src_client, dst_counter, dst_client, trigger, animation) {
        const ptr0 = passStringToWasm0(trigger, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(animation, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_prototype_link(this.__wbg_ptr, src_counter, src_client, dst_counter, dst_client, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Add a rectangle. Returns node ID as [counter, client_id].
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {Uint32Array}
     */
    add_rectangle(name, x, y, width, height, r, g, b, a) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_rectangle(this.__wbg_ptr, ptr0, len0, x, y, width, height, r, g, b, a);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Add a rounded rectangle. Returns node ID as [counter, client_id].
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} radius
     * @returns {Uint32Array}
     */
    add_rounded_rect(name, x, y, width, height, r, g, b, a, radius) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_rounded_rect(this.__wbg_ptr, ptr0, len0, x, y, width, height, r, g, b, a, radius);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Add a star/polygon. `points` = number of outer points (3=triangle, 5=star, 6=hexagon).
     * `inner_ratio` = inner radius / outer radius (0.0..1.0). Use 1.0 for regular polygon.
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} points
     * @param {number} inner_ratio
     * @returns {Uint32Array}
     */
    add_star(name, x, y, width, height, r, g, b, a, points, inner_ratio) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_star(this.__wbg_ptr, ptr0, len0, x, y, width, height, r, g, b, a, points, inner_ratio);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Add a text node. Returns node ID as [counter, client_id].
     * @param {string} name
     * @param {string} content
     * @param {number} x
     * @param {number} y
     * @param {number} font_size
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {Uint32Array}
     */
    add_text(name, content, x, y, font_size, r, g, b, a) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_text(this.__wbg_ptr, ptr0, len0, ptr1, len1, x, y, font_size, r, g, b, a);
        var v3 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v3;
    }
    /**
     * Add a vector shape from flat path data.
     * Format: each command is [type, ...args]
     *   0, x, y           = MoveTo
     *   1, x, y           = LineTo
     *   2, c1x, c1y, c2x, c2y, x, y = CubicTo
     *   3                 = Close
     * `width`/`height` = bounding box for hit-testing.
     * @param {string} name
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {Float32Array} path_data
     * @returns {Uint32Array}
     */
    add_vector(name, x, y, width, height, r, g, b, a, path_data) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(path_data, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_add_vector(this.__wbg_ptr, ptr0, len0, x, y, width, height, r, g, b, a, ptr1, len1);
        var v3 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v3;
    }
    /**
     * Align selected nodes. direction: 0=left, 1=center-h, 2=right, 3=top, 4=center-v, 5=bottom
     * @param {number} direction
     * @returns {boolean}
     */
    align_selected(direction) {
        const ret = wasm.canvasengine_align_selected(this.__wbg_ptr, direction);
        return ret !== 0;
    }
    /**
     * Apply remote operations (JSON array of Operation).
     * Returns number of ops applied.
     * @param {string} json
     * @returns {number}
     */
    apply_remote_ops(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_apply_remote_ops(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Combine selected nodes with a boolean operation.
     * Creates a BooleanOp parent, moves selected nodes under it.
     * op: 0=Union, 1=Subtract, 2=Intersect, 3=Exclude
     * @param {number} op
     * @returns {boolean}
     */
    boolean_op(op) {
        const ret = wasm.canvasengine_boolean_op(this.__wbg_ptr, op);
        return ret !== 0;
    }
    /**
     * Bring selected nodes forward one step in z-order.
     * @returns {boolean}
     */
    bring_forward() {
        const ret = wasm.canvasengine_bring_forward(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Bring selected nodes to front (top of z-order within their parent).
     * @returns {boolean}
     */
    bring_to_front() {
        const ret = wasm.canvasengine_bring_to_front(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Cancel creation mode.
     */
    cancel_creating() {
        wasm.canvasengine_cancel_creating(this.__wbg_ptr);
    }
    /**
     * Clear insert parent — subsequent adds go to page root.
     */
    clear_insert_parent() {
        wasm.canvasengine_clear_insert_parent(this.__wbg_ptr);
    }
    /**
     * Remove all point clouds and free GPU resources.
     * @param {WebGL2RenderingContext} gl
     */
    clear_point_clouds(gl) {
        wasm.canvasengine_clear_point_clouds(this.__wbg_ptr, gl);
    }
    /**
     * Get comment count.
     * @returns {number}
     */
    comment_count() {
        const ret = wasm.canvasengine_comment_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Copy selected nodes to internal clipboard.
     * @returns {number}
     */
    copy_selected() {
        const ret = wasm.canvasengine_copy_selected(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create a component from selected nodes (wraps them like group, but NodeKind::Component).
     * Returns component node ID as [counter, client_id], or empty on failure.
     * @returns {Uint32Array}
     */
    create_component() {
        const ret = wasm.canvasengine_create_component(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Create an instance of a component. Deep-clones the component's children.
     * Returns instance node ID as [counter, client_id], or empty on failure.
     * @param {number} comp_counter
     * @param {number} comp_client_id
     * @returns {Uint32Array}
     */
    create_instance(comp_counter, comp_client_id) {
        const ret = wasm.canvasengine_create_instance(this.__wbg_ptr, comp_counter, comp_client_id);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get current page index.
     * @returns {number}
     */
    current_page_index() {
        const ret = wasm.canvasengine_current_page_index(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Delete a comment by ID.
     * @param {number} comment_id
     * @returns {boolean}
     */
    delete_comment(comment_id) {
        const ret = wasm.canvasengine_delete_comment(this.__wbg_ptr, comment_id);
        return ret !== 0;
    }
    /**
     * Delete all selected nodes.
     * @returns {boolean}
     */
    delete_selected() {
        const ret = wasm.canvasengine_delete_selected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Detach an instance: convert it to a plain Frame, keeping its children.
     * Returns true on success.
     * @returns {boolean}
     */
    detach_instance() {
        const ret = wasm.canvasengine_detach_instance(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Distribute selected nodes evenly. direction: 0=horizontal, 1=vertical
     * @param {number} direction
     * @returns {boolean}
     */
    distribute_selected(direction) {
        const ret = wasm.canvasengine_distribute_selected(this.__wbg_ptr, direction);
        return ret !== 0;
    }
    /**
     * Number of items drawn in last render frame (for diagnostics).
     * @returns {number}
     */
    drawn_count() {
        const ret = wasm.canvasengine_drawn_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Duplicate selected nodes in-place (copy + paste in one step).
     * @returns {number}
     */
    duplicate_selected() {
        const ret = wasm.canvasengine_duplicate_selected(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Exit the currently entered group. Selects the group itself.
     */
    exit_group() {
        wasm.canvasengine_exit_group(this.__wbg_ptr);
    }
    /**
     * Export the entire document as JSON for persistence.
     * @returns {string}
     */
    export_document_json() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_export_document_json(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Export the canvas at 1:1 scale without selection overlay.
     * Returns raw RGBA pixel data. JS converts to PNG via canvas.
     * @param {number} width
     * @param {number} height
     * @returns {Uint8Array}
     */
    export_pixels(width, height) {
        const ret = wasm.canvasengine_export_pixels(this.__wbg_ptr, width, height);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Export the current page as SVG string.
     * @param {number} width
     * @param {number} height
     * @returns {string}
     */
    export_svg(width, height) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_export_svg(this.__wbg_ptr, width, height);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Find nodes by name substring. Returns JSON array of {counter, client_id, name, info}.
     * @param {string} query
     * @returns {string}
     */
    find_nodes_by_name(query) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_find_nodes_by_name(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Find nodes that use a specific image key. Returns JSON array of {counter, client_id, name}.
     * @param {string} image_key
     * @returns {string}
     */
    find_nodes_with_image(image_key) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(image_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_find_nodes_with_image(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Flatten selected node to a vector path (Cmd+E).
     * Converts rectangles, ellipses, polygons, etc. to their path representation.
     * Returns true on success.
     * @returns {boolean}
     */
    flatten_selected() {
        const ret = wasm.canvasengine_flatten_selected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get all image assets on the current page.
     * Returns JSON array: [{type:"node"|"fill", key:string, name:string, counter:u64, client_id:u32}]
     * "node" = NodeKind::Image (raw pixels), "fill" = Paint::Image (referenced by path).
     * @returns {string}
     */
    get_all_image_keys() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_all_image_keys(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get current camera state as [cam_x, cam_y, zoom].
     * @returns {Float32Array}
     */
    get_camera() {
        const ret = wasm.canvasengine_get_camera(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get all comments as JSON array.
     * @returns {string}
     */
    get_comments() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_comments(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get creation preview rectangle [x, y, w, h] in world coords during drag.
     * Returns empty vec if not currently dragging a creation.
     * @returns {Float32Array}
     */
    get_creation_preview() {
        const ret = wasm.canvasengine_get_creation_preview(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Returns the entered group's counter and client_id, or (-1, -1) if none.
     * @returns {BigInt64Array}
     */
    get_entered_group() {
        const ret = wasm.canvasengine_get_entered_group(this.__wbg_ptr);
        var v1 = getArrayI64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * Get image bytes extracted from a .fig ZIP by path.
     * Returns the raw PNG/JPEG bytes, or empty vec if not found.
     * @param {string} path
     * @returns {Uint8Array}
     */
    get_imported_image(path) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_get_imported_image(this.__wbg_ptr, ptr0, len0);
        var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v2;
    }
    /**
     * Get layer list as JSON array: [{"id":[counter,client_id],"name":"..."}]
     * @returns {string}
     */
    get_layers() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_layers(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get a range of layers as JSON: [{"id":[counter,client_id],"name":"..."}]
     * `start` is 0-based index, `count` is max items to return.
     * @param {number} start
     * @param {number} count
     * @returns {string}
     */
    get_layers_range(start, count) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_layers_range(this.__wbg_ptr, start, count);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Returns the current marquee selection rectangle in world coords, or empty if not dragging.
     * Format: [min_x, min_y, max_x, max_y]. Used by TypeScript to draw the selection overlay.
     * @returns {Float32Array}
     */
    get_marquee_rect() {
        const ret = wasm.canvasengine_get_marquee_rect(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get properties of the selected node as JSON.
     * Returns empty string if nothing is selected.
     * @param {number} counter
     * @param {number} client_id
     * @returns {string}
     */
    get_node_info(counter, client_id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_node_info(this.__wbg_ptr, counter, client_id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get a node's name by ID. Returns empty string if not found.
     * @param {number} counter
     * @param {number} client_id
     * @returns {string}
     */
    get_node_name(counter, client_id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_node_name(this.__wbg_ptr, counter, client_id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get a node's world-space bounding box: [x, y, width, height].
     * Accounts for all parent transforms (works at any nesting depth).
     * @param {number} counter
     * @param {number} client_id
     * @returns {Float32Array}
     */
    get_node_world_bounds(counter, client_id) {
        const ret = wasm.canvasengine_get_node_world_bounds(this.__wbg_ptr, counter, client_id);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get pages as JSON: [{"index":0,"name":"Page 1"},...]
     * @returns {string}
     */
    get_pages() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_pages(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get pending ops as JSON and clear the queue.
     * @returns {string}
     */
    get_pending_ops() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_pending_ops(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get all prototype links as JSON array.
     * @returns {string}
     */
    get_prototype_links() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_prototype_links(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get selected node IDs. Returns flat array: [counter0, client0, counter1, client1, ...].
     * @returns {Uint32Array}
     */
    get_selected() {
        const ret = wasm.canvasengine_get_selected(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get current snap grid size.
     * @returns {number}
     */
    get_snap_grid() {
        const ret = wasm.canvasengine_get_snap_grid(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get text-on-arc parameters for a node. Returns [radius, start_angle, letter_spacing] or empty.
     * @param {number} counter
     * @param {number} client_id
     * @returns {Float32Array}
     */
    get_text_arc(counter, client_id) {
        const ret = wasm.canvasengine_get_text_arc(this.__wbg_ptr, counter, client_id);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get layer tree as flat DFS list with depth info.
     * `expanded_ids` is comma-separated "counter:client" pairs for expanded nodes.
     * Returns JSON: [{"id":[c,cl],"name":"...","depth":N,"hasChildren":bool,"kind":"frame"|...}]
     * Only descends into expanded nodes. Supports virtualized rendering.
     * @param {string} expanded_ids
     * @param {number} start
     * @param {number} count
     * @returns {Uint32Array}
     */
    get_tree_layers(expanded_ids, start, count) {
        const ptr0 = passStringToWasm0(expanded_ids, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_get_tree_layers(this.__wbg_ptr, ptr0, len0, start, count);
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * Get vector network data for a vector node as JSON.
     * Returns vertices + segments representation (graph-based, not sequential paths).
     * This is the Figma vector network format: vertices share positions,
     * segments connect pairs of vertices with bezier tangent handles.
     * @param {number} counter
     * @param {number} client_id
     * @returns {string}
     */
    get_vector_network(counter, client_id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_vector_network(this.__wbg_ptr, counter, client_id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get image fills visible in the current viewport as JSON.
     * Returns: [[path, screenX, screenY, screenW, screenH, opacity], ...]
     * JS uses this to overlay HTMLImageElements after WASM renders the scene.
     * @param {number} width
     * @param {number} height
     * @returns {string}
     */
    get_visible_image_fills(width, height) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_get_visible_image_fills(this.__wbg_ptr, width, height);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Group selected nodes into a Frame.
     * @returns {boolean}
     */
    group_selected() {
        const ret = wasm.canvasengine_group_selected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Handle explicit double-click from browser dblclick event.
     * Enters group or vector editing mode for the node under cursor.
     * This avoids timing-based double-click detection which can fail
     * when the browser event loop adds latency between mousedown events.
     * @param {number} sx
     * @param {number} sy
     * @returns {boolean}
     */
    handle_double_click(sx, sy) {
        const ret = wasm.canvasengine_handle_double_click(this.__wbg_ptr, sx, sy);
        return ret !== 0;
    }
    /**
     * Import a document from JSON snapshot, replacing the current document.
     * Returns status JSON: {"ok":true,"pages":N,"nodes":N} or {"ok":false,"error":"..."}
     * @param {string} json
     * @returns {string}
     */
    import_document_json(json) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_import_document_json(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Import a .fig binary directly. No external tools needed.
     * Returns JSON: {"pages":N,"nodes":N,"images":[path,...],"errors":[...]}
     * @param {Uint8Array} bytes
     * @returns {string}
     */
    import_fig_binary(bytes) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_import_fig_binary(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Import a .fig file's JSON (from fig2json) into the document.
     * Returns JSON: {"pages": N, "nodes": N, "errors": [...]}
     * @param {string} json_str
     * @param {string} image_base
     * @returns {string}
     */
    import_fig_json(json_str, image_base) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(json_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(image_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_import_fig_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Import a single page from fig JSON (for large files).
     * JS should parse the full JSON, extract each page object, and stringify it individually.
     * @param {string} page_json
     * @param {string} image_base
     * @returns {string}
     */
    import_fig_page_json(page_json, image_base) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(page_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(image_base, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.canvasengine_import_fig_page_json(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Whether we're in creation mode (waiting for mousedown).
     * @returns {boolean}
     */
    is_creating() {
        const ret = wasm.canvasengine_is_creating(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if screen coords are in the rotation zone (outside resize handles).
     * @param {number} sx
     * @param {number} sy
     * @returns {boolean}
     */
    is_rotation_zone(sx, sy) {
        const ret = wasm.canvasengine_is_rotation_zone(this.__wbg_ptr, sx, sy);
        return ret !== 0;
    }
    /**
     * Whether we're in vector point editing mode.
     * @returns {boolean}
     */
    is_vector_editing() {
        const ret = wasm.canvasengine_is_vector_editing(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Total number of layers (children of root).
     * @returns {number}
     */
    layer_count() {
        const ret = wasm.canvasengine_layer_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Handle mouse down. Coordinates are SCREEN space.
     * shift=true adds/removes from selection instead of replacing.
     * Returns true if something was selected.
     * @param {number} sx
     * @param {number} sy
     * @param {boolean} shift
     * @returns {boolean}
     */
    mouse_down(sx, sy, shift) {
        const ret = wasm.canvasengine_mouse_down(this.__wbg_ptr, sx, sy, shift);
        return ret !== 0;
    }
    /**
     * Handle mouse move (drag/resize). Coordinates are SCREEN space.
     * @param {number} sx
     * @param {number} sy
     */
    mouse_move(sx, sy) {
        wasm.canvasengine_mouse_move(this.__wbg_ptr, sx, sy);
    }
    /**
     * Handle mouse up. Emits CRDT ops for any drag/resize that happened.
     */
    mouse_up() {
        wasm.canvasengine_mouse_up(this.__wbg_ptr);
    }
    /**
     * Check if a re-render is needed.
     * @returns {boolean}
     */
    needs_render() {
        const ret = wasm.canvasengine_needs_render(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {string} name
     * @param {number} client_id
     */
    constructor(name, client_id) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_new(ptr0, len0, client_id);
        this.__wbg_ptr = ret >>> 0;
        CanvasEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    node_count() {
        const ret = wasm.canvasengine_node_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get number of pages.
     * @returns {number}
     */
    page_count() {
        const ret = wasm.canvasengine_page_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Stop panning.
     */
    pan_end() {
        wasm.canvasengine_pan_end(this.__wbg_ptr);
    }
    /**
     * Continue panning.
     * @param {number} screen_x
     * @param {number} screen_y
     */
    pan_move(screen_x, screen_y) {
        wasm.canvasengine_pan_move(this.__wbg_ptr, screen_x, screen_y);
    }
    /**
     * Start panning (called on middle-click down or space+click).
     * @param {number} screen_x
     * @param {number} screen_y
     */
    pan_start(screen_x, screen_y) {
        wasm.canvasengine_pan_start(this.__wbg_ptr, screen_x, screen_y);
    }
    /**
     * Paste clipboard nodes offset by (10,10). Selects the pasted nodes.
     * @returns {number}
     */
    paste() {
        const ret = wasm.canvasengine_paste(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Cancel pen tool and discard the path.
     */
    pen_cancel() {
        wasm.canvasengine_pen_cancel(this.__wbg_ptr);
    }
    /**
     * Finish pen path as closed path (click on first anchor).
     */
    pen_finish_closed() {
        wasm.canvasengine_pen_finish_closed(this.__wbg_ptr);
    }
    /**
     * Finish pen path as open path (double-click or Enter).
     */
    pen_finish_open() {
        wasm.canvasengine_pen_finish_open(this.__wbg_ptr);
    }
    /**
     * Get pen path data for overlay rendering.
     * Returns JSON: { anchors: [{x,y,hox,hoy,hix,hiy}], cursor: {x,y}, closed: false }
     * @returns {string}
     */
    pen_get_state() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_pen_get_state(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Is the pen tool currently active?
     * @returns {boolean}
     */
    pen_is_active() {
        const ret = wasm.canvasengine_pen_is_active(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Mouse down in pen mode (screen coords). Adds an anchor.
     * If clicking near the first anchor, closes the path.
     * @param {number} sx
     * @param {number} sy
     */
    pen_mouse_down(sx, sy) {
        wasm.canvasengine_pen_mouse_down(this.__wbg_ptr, sx, sy);
    }
    /**
     * Mouse drag in pen mode (screen coords). Creates curve handles.
     * @param {number} sx
     * @param {number} sy
     */
    pen_mouse_drag(sx, sy) {
        wasm.canvasengine_pen_mouse_drag(this.__wbg_ptr, sx, sy);
    }
    /**
     * Mouse move in pen mode (for preview line).
     * @param {number} sx
     * @param {number} sy
     */
    pen_mouse_move(sx, sy) {
        wasm.canvasengine_pen_mouse_move(this.__wbg_ptr, sx, sy);
    }
    /**
     * Mouse up in pen mode.
     */
    pen_mouse_up() {
        wasm.canvasengine_pen_mouse_up(this.__wbg_ptr);
    }
    /**
     * Enter pen drawing mode.
     */
    pen_start() {
        wasm.canvasengine_pen_start(this.__wbg_ptr);
    }
    /**
     * Total number of points across all point clouds.
     * @returns {number}
     */
    point_cloud_count() {
        const ret = wasm.canvasengine_point_cloud_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get prototype link count.
     * @returns {number}
     */
    prototype_link_count() {
        const ret = wasm.canvasengine_prototype_link_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Redo the last undone action. Returns true if something was redone.
     * @returns {boolean}
     */
    redo() {
        const ret = wasm.canvasengine_redo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Remove auto-layout from a frame.
     * @param {number} counter
     * @param {number} client_id
     * @returns {boolean}
     */
    remove_auto_layout(counter, client_id) {
        const ret = wasm.canvasengine_remove_auto_layout(this.__wbg_ptr, counter, client_id);
        return ret !== 0;
    }
    /**
     * Remove all strokes from a node.
     * @param {number} counter
     * @param {number} client_id
     * @returns {boolean}
     */
    remove_node_stroke(counter, client_id) {
        const ret = wasm.canvasengine_remove_node_stroke(this.__wbg_ptr, counter, client_id);
        return ret !== 0;
    }
    /**
     * Remove all prototype links from a source node.
     * @param {number} counter
     * @param {number} client_id
     * @returns {boolean}
     */
    remove_prototype_links(counter, client_id) {
        const ret = wasm.canvasengine_remove_prototype_links(this.__wbg_ptr, counter, client_id);
        return ret !== 0;
    }
    /**
     * Rename a page.
     * @param {number} index
     * @param {string} name
     * @returns {boolean}
     */
    rename_page(index, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_rename_page(this.__wbg_ptr, index, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Raster render — returns raw RGBA pixel buffer. Used for PNG export and fallback.
     * @param {number} width
     * @param {number} height
     * @returns {Uint8Array}
     */
    render(width, height) {
        const ret = wasm.canvasengine_render(this.__wbg_ptr, width, height);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Canvas 2D vector render — draws directly to a browser canvas context.
     * GPU-accelerated, no pixel buffer transfer.
     * `dpr` is the device pixel ratio for crisp Retina rendering.
     * @param {CanvasRenderingContext2D} ctx
     * @param {number} width
     * @param {number} height
     * @param {number} dpr
     */
    render_canvas2d(ctx, width, height, dpr) {
        wasm.canvasengine_render_canvas2d(this.__wbg_ptr, ctx, width, height, dpr);
    }
    /**
     * WebGL2 instanced render — batches Rects and Ellipses into 2 GPU draw calls.
     * 10-50x faster than Canvas2D for data-dense scenes (10K+ visible shapes).
     * @param {WebGL2RenderingContext} gl
     * @param {number} width
     * @param {number} height
     * @param {number} dpr
     */
    render_webgl(gl, width, height, dpr) {
        wasm.canvasengine_render_webgl(this.__wbg_ptr, gl, width, height, dpr);
    }
    /**
     * Resolve or unresolve a comment.
     * @param {number} comment_id
     * @param {boolean} resolved
     * @returns {boolean}
     */
    resolve_comment(comment_id, resolved) {
        const ret = wasm.canvasengine_resolve_comment(this.__wbg_ptr, comment_id, resolved);
        return ret !== 0;
    }
    /**
     * Select all direct children of the current page root.
     */
    select_all() {
        wasm.canvasengine_select_all(this.__wbg_ptr);
    }
    /**
     * Select a node by ID (from layers panel click). Replaces current selection.
     * @param {number} counter
     * @param {number} client_id
     */
    select_node(counter, client_id) {
        wasm.canvasengine_select_node(this.__wbg_ptr, counter, client_id);
    }
    /**
     * Send selected nodes backward one step in z-order.
     * @returns {boolean}
     */
    send_backward() {
        const ret = wasm.canvasengine_send_backward(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Send selected nodes to back (bottom of z-order within their parent).
     * @returns {boolean}
     */
    send_to_back() {
        const ret = wasm.canvasengine_send_to_back(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Set auto-layout on a frame node.
     * direction: 0=Horizontal, 1=Vertical
     * After setting, compute_layout is called to position children.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} direction
     * @param {number} spacing
     * @param {number} pad_top
     * @param {number} pad_right
     * @param {number} pad_bottom
     * @param {number} pad_left
     * @returns {boolean}
     */
    set_auto_layout(counter, client_id, direction, spacing, pad_top, pad_right, pad_bottom, pad_left) {
        const ret = wasm.canvasengine_set_auto_layout(this.__wbg_ptr, counter, client_id, direction, spacing, pad_top, pad_right, pad_bottom, pad_left);
        return ret !== 0;
    }
    /**
     * Set camera position and zoom directly.
     * @param {number} x
     * @param {number} y
     * @param {number} zoom
     */
    set_camera(x, y, zoom) {
        wasm.canvasengine_set_camera(this.__wbg_ptr, x, y, zoom);
    }
    /**
     * Set dash pattern on a node's stroke.
     * @param {number} counter
     * @param {number} client_id
     * @param {Float32Array} dashes
     * @returns {boolean}
     */
    set_dash_pattern(counter, client_id, dashes) {
        const ptr0 = passArrayF32ToWasm0(dashes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_dash_pattern(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set image fill on an existing node (replace all fills with an image fill).
     * @param {number} counter
     * @param {number} client_id
     * @param {string} path
     * @param {string} scale_mode
     * @param {number} opacity
     * @returns {boolean}
     */
    set_image_fill(counter, client_id, path, scale_mode, opacity) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(scale_mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_image_fill(this.__wbg_ptr, counter, client_id, ptr0, len0, ptr1, len1, opacity);
        return ret !== 0;
    }
    /**
     * Set parent for subsequent add_* calls (children go inside this node).
     * @param {number} counter
     * @param {number} client_id
     */
    set_insert_parent(counter, client_id) {
        wasm.canvasengine_set_insert_parent(this.__wbg_ptr, counter, client_id);
    }
    /**
     * Set letter spacing on a text node (in pixels).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} spacing
     * @returns {boolean}
     */
    set_letter_spacing(counter, client_id, spacing) {
        const ret = wasm.canvasengine_set_letter_spacing(this.__wbg_ptr, counter, client_id, spacing);
        return ret !== 0;
    }
    /**
     * Set line height on a text node (in pixels, 0 = auto).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} height
     * @returns {boolean}
     */
    set_line_height(counter, client_id, height) {
        const ret = wasm.canvasengine_set_line_height(this.__wbg_ptr, counter, client_id, height);
        return ret !== 0;
    }
    /**
     * Set angular (conic) gradient fill on any node. Replaces existing fills.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} center_x
     * @param {number} center_y
     * @param {number} start_angle
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    set_node_angular_gradient(counter, client_id, center_x, center_y, start_angle, stop_positions, stop_colors) {
        const ptr0 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_angular_gradient(this.__wbg_ptr, counter, client_id, center_x, center_y, start_angle, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Set blend mode on a node. mode: 0=Normal, 1=Multiply, 2=Screen, 3=Overlay,
     * 4=Darken, 5=Lighten, 6=ColorDodge, 7=ColorBurn, 8=HardLight, 9=SoftLight,
     * 10=Difference, 11=Exclusion, 12=Hue, 13=Saturation, 14=Color, 15=Luminosity.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} mode
     * @returns {boolean}
     */
    set_node_blend_mode(counter, client_id, mode) {
        const ret = wasm.canvasengine_set_node_blend_mode(this.__wbg_ptr, counter, client_id, mode);
        return ret !== 0;
    }
    /**
     * Set constraints on a node. h: 0=left, 1=right, 2=leftRight, 3=center, 4=scale
     * v: 0=top, 1=bottom, 2=topBottom, 3=center, 4=scale
     * @param {number} counter
     * @param {number} client_id
     * @param {number} h
     * @param {number} v
     * @returns {boolean}
     */
    set_node_constraints(counter, client_id, h, v) {
        const ret = wasm.canvasengine_set_node_constraints(this.__wbg_ptr, counter, client_id, h, v);
        return ret !== 0;
    }
    /**
     * Set corner radius on a rectangle or frame node.
     * If all four values are the same, uses uniform radius. Otherwise per-corner.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} tl
     * @param {number} tr
     * @param {number} br
     * @param {number} bl
     * @returns {boolean}
     */
    set_node_corner_radius(counter, client_id, tl, tr, br, bl) {
        const ret = wasm.canvasengine_set_node_corner_radius(this.__wbg_ptr, counter, client_id, tl, tr, br, bl);
        return ret !== 0;
    }
    /**
     * Set node fill color (RGBA 0-1 range).
     * For text nodes, also updates the per-run text color.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {boolean}
     */
    set_node_fill(counter, client_id, r, g, b, a) {
        const ret = wasm.canvasengine_set_node_fill(this.__wbg_ptr, counter, client_id, r, g, b, a);
        return ret !== 0;
    }
    /**
     * Set all fills on a node from a JSON array. Handles solid, gradient, and image fills.
     * JSON format: [{"type":"solid","r":255,"g":0,"b":0,"a":1.0}, {"type":"linear","startX":0,...,"stops":[...]}, ...]
     * @param {number} counter
     * @param {number} client_id
     * @param {string} fills_json
     * @returns {boolean}
     */
    set_node_fills_json(counter, client_id, fills_json) {
        const ptr0 = passStringToWasm0(fills_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_fills_json(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set font family on a text node (e.g. "Inter", "Roboto", "Poppins").
     * @param {number} counter
     * @param {number} client_id
     * @param {string} family
     * @returns {boolean}
     */
    set_node_font_family(counter, client_id, family) {
        const ptr0 = passStringToWasm0(family, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_font_family(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set font size of a text node.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} size
     * @returns {boolean}
     */
    set_node_font_size(counter, client_id, size) {
        const ret = wasm.canvasengine_set_node_font_size(this.__wbg_ptr, counter, client_id, size);
        return ret !== 0;
    }
    /**
     * Set font weight on a text node (300=Light, 400=Regular, 500=Medium, 600=Semibold, 700=Bold).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} weight
     * @returns {boolean}
     */
    set_node_font_weight(counter, client_id, weight) {
        const ret = wasm.canvasengine_set_node_font_weight(this.__wbg_ptr, counter, client_id, weight);
        return ret !== 0;
    }
    /**
     * Set linear gradient fill on any node. Replaces existing fills.
     * start/end are in 0..1 normalized coordinates (relative to node bounds).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} start_x
     * @param {number} start_y
     * @param {number} end_x
     * @param {number} end_y
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    set_node_linear_gradient(counter, client_id, start_x, start_y, end_x, end_y, stop_positions, stop_colors) {
        const ptr0 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_linear_gradient(this.__wbg_ptr, counter, client_id, start_x, start_y, end_x, end_y, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Set or unset the mask flag on a node.
     * When true, the node's shape clips all subsequent siblings until the parent ends.
     * @param {number} counter
     * @param {number} client_id
     * @param {boolean} is_mask
     * @returns {boolean}
     */
    set_node_mask(counter, client_id, is_mask) {
        const ret = wasm.canvasengine_set_node_mask(this.__wbg_ptr, counter, client_id, is_mask);
        return ret !== 0;
    }
    /**
     * Set node name.
     * @param {number} counter
     * @param {number} client_id
     * @param {string} name
     * @returns {boolean}
     */
    set_node_name(counter, client_id, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_name(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set opacity on a node (0.0 to 1.0).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} opacity
     * @returns {boolean}
     */
    set_node_opacity(counter, client_id, opacity) {
        const ret = wasm.canvasengine_set_node_opacity(this.__wbg_ptr, counter, client_id, opacity);
        return ret !== 0;
    }
    /**
     * Set node position from the properties panel.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    set_node_position(counter, client_id, x, y) {
        const ret = wasm.canvasengine_set_node_position(this.__wbg_ptr, counter, client_id, x, y);
        return ret !== 0;
    }
    /**
     * Set radial gradient fill on any node. Replaces existing fills.
     * center is in 0..1 normalized coordinates. radius is 0..1 (1.0 = full extent).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} center_x
     * @param {number} center_y
     * @param {number} radius
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    set_node_radial_gradient(counter, client_id, center_x, center_y, radius, stop_positions, stop_colors) {
        const ptr0 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_radial_gradient(this.__wbg_ptr, counter, client_id, center_x, center_y, radius, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * Set node rotation in degrees. Preserves scale.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} degrees
     * @returns {boolean}
     */
    set_node_rotation(counter, client_id, degrees) {
        const ret = wasm.canvasengine_set_node_rotation(this.__wbg_ptr, counter, client_id, degrees);
        return ret !== 0;
    }
    /**
     * Set node size from the properties panel.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} w
     * @param {number} h
     * @returns {boolean}
     */
    set_node_size(counter, client_id, w, h) {
        const ret = wasm.canvasengine_set_node_size(this.__wbg_ptr, counter, client_id, w, h);
        return ret !== 0;
    }
    /**
     * Set stroke on a node (color + weight). Replaces all existing strokes.
     * @param {number} counter
     * @param {number} client_id
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} weight
     * @returns {boolean}
     */
    set_node_stroke(counter, client_id, r, g, b, a, weight) {
        const ret = wasm.canvasengine_set_node_stroke(this.__wbg_ptr, counter, client_id, r, g, b, a, weight);
        return ret !== 0;
    }
    /**
     * Set the text content of a text node.
     * @param {number} counter
     * @param {number} client_id
     * @param {string} text
     * @returns {boolean}
     */
    set_node_text(counter, client_id, text) {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_node_text(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set snap-to-grid size. 0 = disabled, typical values: 1, 4, 8, 16, 32.
     * @param {number} size
     */
    set_snap_grid(size) {
        wasm.canvasengine_set_snap_grid(this.__wbg_ptr, size);
    }
    /**
     * Set stroke alignment: "inside", "center", or "outside".
     * @param {number} counter
     * @param {number} client_id
     * @param {string} align
     * @returns {boolean}
     */
    set_stroke_align(counter, client_id, align) {
        const ptr0 = passStringToWasm0(align, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_stroke_align(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set text horizontal alignment: "left", "center", "right".
     * @param {number} counter
     * @param {number} client_id
     * @param {string} align
     * @returns {boolean}
     */
    set_text_align(counter, client_id, align) {
        const ptr0 = passStringToWasm0(align, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_text_align(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set text-on-arc parameters. radius=0 removes arc rendering.
     * start_angle is in radians (−PI/2 = top of circle, PI/2 = bottom).
     * @param {number} counter
     * @param {number} client_id
     * @param {number} radius
     * @param {number} start_angle
     * @param {number} letter_spacing
     */
    set_text_arc(counter, client_id, radius, start_angle, letter_spacing) {
        wasm.canvasengine_set_text_arc(this.__wbg_ptr, counter, client_id, radius, start_angle, letter_spacing);
    }
    /**
     * Set text decoration: "none", "underline", or "strikethrough".
     * @param {number} counter
     * @param {number} client_id
     * @param {string} decoration
     * @returns {boolean}
     */
    set_text_decoration(counter, client_id, decoration) {
        const ptr0 = passStringToWasm0(decoration, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_text_decoration(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Set gradient fill on text (all runs). Type: "linear" or "radial".
     * For linear: extra = [start_x, start_y, end_x, end_y]. For radial: extra = [center_x, center_y, radius].
     * @param {number} counter
     * @param {number} client_id
     * @param {string} gradient_type
     * @param {Float32Array} extra
     * @param {Float32Array} stop_positions
     * @param {Float32Array} stop_colors
     * @returns {boolean}
     */
    set_text_gradient_fill(counter, client_id, gradient_type, extra, stop_positions, stop_colors) {
        const ptr0 = passStringToWasm0(gradient_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(extra, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArrayF32ToWasm0(stop_positions, wasm.__wbindgen_malloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passArrayF32ToWasm0(stop_colors, wasm.__wbindgen_malloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_text_gradient_fill(this.__wbg_ptr, counter, client_id, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        return ret !== 0;
    }
    /**
     * Set text vertical alignment: "top", "center", or "bottom".
     * @param {number} counter
     * @param {number} client_id
     * @param {string} align
     * @returns {boolean}
     */
    set_text_vertical_align(counter, client_id, align) {
        const ptr0 = passStringToWasm0(align, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.canvasengine_set_text_vertical_align(this.__wbg_ptr, counter, client_id, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {number} width
     * @param {number} height
     */
    set_viewport(width, height) {
        wasm.canvasengine_set_viewport(this.__wbg_ptr, width, height);
    }
    /**
     * Start shape creation mode. Next mousedown+drag will create the shape.
     * shape_type: "rect", "ellipse", "frame", "star", "text"
     * @param {string} shape_type
     */
    start_creating(shape_type) {
        const ptr0 = passStringToWasm0(shape_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.canvasengine_start_creating(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Switch to a different page by index.
     * @param {number} index
     * @returns {boolean}
     */
    switch_page(index) {
        const ret = wasm.canvasengine_switch_page(this.__wbg_ptr, index);
        return ret !== 0;
    }
    /**
     * Toggle a node in/out of the selection (shift-click in layers panel).
     * @param {number} counter
     * @param {number} client_id
     */
    toggle_select_node(counter, client_id) {
        wasm.canvasengine_toggle_select_node(this.__wbg_ptr, counter, client_id);
    }
    /**
     * Undo the last action. Returns true if something was undone.
     * @returns {boolean}
     */
    undo() {
        const ret = wasm.canvasengine_undo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Ungroup: move children of selected group to its parent, remove the group.
     * @returns {boolean}
     */
    ungroup_selected() {
        const ret = wasm.canvasengine_ungroup_selected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Exit vector editing mode (from JS, e.g. Escape key).
     */
    vector_edit_exit() {
        wasm.canvasengine_vector_edit_exit(this.__wbg_ptr);
    }
    /**
     * Get vector edit state as JSON for overlay rendering.
     * Returns: {"anchors":[{x,y,hox,hoy,hix,hiy}],"selected":N,"closed":bool,"tx":F,"ty":F}
     * @returns {string}
     */
    vector_edit_get_state() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.canvasengine_vector_edit_get_state(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Zoom centered on a screen point. delta > 0 zooms in, < 0 zooms out.
     * @param {number} delta
     * @param {number} screen_x
     * @param {number} screen_y
     */
    zoom(delta, screen_x, screen_y) {
        wasm.canvasengine_zoom(this.__wbg_ptr, delta, screen_x, screen_y);
    }
    /**
     * Zoom to fit all content on the current page.
     * @returns {boolean}
     */
    zoom_to_fit() {
        const ret = wasm.canvasengine_zoom_to_fit(this.__wbg_ptr);
        return ret !== 0;
    }
}
if (Symbol.dispose) CanvasEngine.prototype[Symbol.dispose] = CanvasEngine.prototype.free;

export class FigmaBench {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        FigmaBenchFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_figmabench_free(ptr, 0);
    }
    /**
     * Return a zero-copy Float32Array view into WASM linear memory.
     * SAFETY: The view is invalidated if WASM memory grows (e.g. new allocations).
     * Caller must use it immediately within the same JS turn.
     * @returns {Float32Array}
     */
    data_view() {
        const ret = wasm.figmabench_data_view(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} count
     * @param {number} width
     * @param {number} height
     */
    constructor(count, width, height) {
        const ret = wasm.figmabench_new(count, width, height);
        this.__wbg_ptr = ret >>> 0;
        FigmaBenchFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    rect_count() {
        const ret = wasm.figmabench_rect_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Update all positions in WASM. Tight loop, no JS calls, no GC.
     */
    update() {
        wasm.figmabench_update(this.__wbg_ptr);
    }
}
if (Symbol.dispose) FigmaBench.prototype[Symbol.dispose] = FigmaBench.prototype.free;

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_c0f3f60bac5a78d1: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_is_undefined_52709e72fb9f179c: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_addColorStop_3bd77f997fb1fa1c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addColorStop(arg1, getStringFromWasm0(arg2, arg3));
        }, arguments); },
        __wbg_apply_d7728efbea08f95e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.apply(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_attachShader_e557f37438249ff7: function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        },
        __wbg_beginPath_596efed55075dbc3: function(arg0) {
            arg0.beginPath();
        },
        __wbg_bezierCurveTo_ee956cad5cea25b2: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.bezierCurveTo(arg1, arg2, arg3, arg4, arg5, arg6);
        },
        __wbg_bindBuffer_142694a9732bc098: function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindVertexArray_c307251f3ff61930: function(arg0, arg1) {
            arg0.bindVertexArray(arg1);
        },
        __wbg_blendFuncSeparate_6aae138b81d75b47: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_bufferData_d20232e3d5dcdc62: function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_clearColor_080c8446c8438f8e: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        },
        __wbg_clearRect_ea4f3d34d76f4bc5: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearRect(arg1, arg2, arg3, arg4);
        },
        __wbg_clear_3d6ad4729e206aac: function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        },
        __wbg_clip_307b93ada960ec4d: function(arg0, arg1) {
            arg0.clip(__wbindgen_enum_CanvasWindingRule[arg1]);
        },
        __wbg_clip_3112b0bb495d0e08: function(arg0) {
            arg0.clip();
        },
        __wbg_closePath_f96bcae0fc7087a9: function(arg0) {
            arg0.closePath();
        },
        __wbg_compileShader_7ca66245c2798601: function(arg0, arg1) {
            arg0.compileShader(arg1);
        },
        __wbg_createBuffer_1aa34315dc9585a2: function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createElement_9b0aab265c549ded: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createLinearGradient_824cc20f7bc01e49: function(arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createLinearGradient(arg1, arg2, arg3, arg4);
            return ret;
        },
        __wbg_createProgram_1fa32901e4db13cd: function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createRadialGradient_5d814c3de73f7596: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            const ret = arg0.createRadialGradient(arg1, arg2, arg3, arg4, arg5, arg6);
            return ret;
        }, arguments); },
        __wbg_createShader_a00913b8c6489e6b: function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createVertexArray_420460898dc8d838: function(arg0) {
            const ret = arg0.createVertexArray();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_deleteBuffer_b053c58b4ed1ab1c: function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        },
        __wbg_deleteProgram_cb8f79d5c1e84863: function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        },
        __wbg_deleteShader_5b6992b5e5894d44: function(arg0, arg1) {
            arg0.deleteShader(arg1);
        },
        __wbg_deleteVertexArray_5a75f4855c2881df: function(arg0, arg1) {
            arg0.deleteVertexArray(arg1);
        },
        __wbg_document_c0320cd4183c6d9b: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_drawArraysInstanced_13e40fca13079ade: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_drawImage_ce7fb4f15446013d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawImage(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_ellipse_96d09af8c5281733: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.ellipse(arg1, arg2, arg3, arg4, arg5, arg6, arg7);
        }, arguments); },
        __wbg_enableVertexAttribArray_60dadea3a00e104a: function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enable_91dff7f43064bb54: function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        },
        __wbg_error_8d9a8e04cd1d3588: function(arg0) {
            console.error(arg0);
        },
        __wbg_fillRect_4e5596ca954226e7: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.fillRect(arg1, arg2, arg3, arg4);
        },
        __wbg_fillText_b1722b6179692b85: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_fill_c0bb5e0ec0d7fcf9: function(arg0) {
            arg0.fill();
        },
        __wbg_fill_e228aca002a2bbf7: function(arg0, arg1) {
            arg0.fill(__wbindgen_enum_CanvasWindingRule[arg1]);
        },
        __wbg_getContext_f04bf8f22dcb2d53: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getProgramInfoLog_50443ddea7475f57: function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramParameter_46e2d49878b56edd: function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getShaderInfoLog_22f9e8c90a52f38d: function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderParameter_46f64f7ca5d534db: function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getUniformLocation_5eb08673afa04eee: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_3ef1eba1850ade27: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_instanceof_CanvasRenderingContext2d_08b9d193c22fa886: function(arg0) {
            let result;
            try {
                result = arg0 instanceof CanvasRenderingContext2D;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_26125339f936be50: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_23e677d2c6843922: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_lineTo_8ea7db5b5d763030: function(arg0, arg1, arg2) {
            arg0.lineTo(arg1, arg2);
        },
        __wbg_linkProgram_b969f67969a850b5: function(arg0, arg1) {
            arg0.linkProgram(arg1);
        },
        __wbg_measureText_a914720e0a913aef: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.measureText(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_moveTo_6d04ca2f71946754: function(arg0, arg1, arg2) {
            arg0.moveTo(arg1, arg2);
        },
        __wbg_new_a70fbab9066b301f: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_with_length_3259a525196bd8cc: function(arg0) {
            const ret = new Array(arg0 >>> 0);
            return ret;
        },
        __wbg_new_with_u8_clamped_array_and_sh_5d9be5b17e50951c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = new ImageData(getClampedArrayU8FromWasm0(arg0, arg1), arg2 >>> 0, arg3 >>> 0);
            return ret;
        }, arguments); },
        __wbg_now_16f0c993d5dd6c27: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_push_e87b0e732085a946: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_putImageData_1750176f4dd07174: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.putImageData(arg1, arg2, arg3);
        }, arguments); },
        __wbg_quadraticCurveTo_79b47836efb75da7: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.quadraticCurveTo(arg1, arg2, arg3, arg4);
        },
        __wbg_rect_9fb7070ab71d27aa: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.rect(arg1, arg2, arg3, arg4);
        },
        __wbg_restore_ec1ece47cce5dc64: function(arg0) {
            arg0.restore();
        },
        __wbg_rotate_326ea70a87136df5: function() { return handleError(function (arg0, arg1) {
            arg0.rotate(arg1);
        }, arguments); },
        __wbg_save_c4e64a4ec29f000f: function(arg0) {
            arg0.save();
        },
        __wbg_setLineDash_b22b8de6051bb23a: function() { return handleError(function (arg0, arg1) {
            arg0.setLineDash(arg1);
        }, arguments); },
        __wbg_setTransform_ad844af0b72d0b8b: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setTransform(arg1, arg2, arg3, arg4, arg5, arg6);
        }, arguments); },
        __wbg_set_282384002438957f: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_7eaa4f96924fd6b3: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_fillStyle_1f65027a07e93e62: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_fillStyle_58417b6b548ae475: function(arg0, arg1, arg2) {
            arg0.fillStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_fillStyle_a48824bb58eba9bd: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_font_b038797b3573ae5e: function(arg0, arg1, arg2) {
            arg0.font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_globalAlpha_d51aa11e10f40cfc: function(arg0, arg1) {
            arg0.globalAlpha = arg1;
        },
        __wbg_set_globalCompositeOperation_ba0da38482e6aa11: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.globalCompositeOperation = getStringFromWasm0(arg1, arg2);
        }, arguments); },
        __wbg_set_height_b6548a01bdcb689a: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_lineCap_1ecf6c7ca9319eb2: function(arg0, arg1, arg2) {
            arg0.lineCap = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineJoin_2c56d0d6bec26d27: function(arg0, arg1, arg2) {
            arg0.lineJoin = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineWidth_e38550ed429ec417: function(arg0, arg1) {
            arg0.lineWidth = arg1;
        },
        __wbg_set_shadowBlur_ceb33c8cba323df6: function(arg0, arg1) {
            arg0.shadowBlur = arg1;
        },
        __wbg_set_shadowColor_10f48b3fd0e00936: function(arg0, arg1, arg2) {
            arg0.shadowColor = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_shadowOffsetX_b67249871fab74ff: function(arg0, arg1) {
            arg0.shadowOffsetX = arg1;
        },
        __wbg_set_shadowOffsetY_a7be739a02e96f34: function(arg0, arg1) {
            arg0.shadowOffsetY = arg1;
        },
        __wbg_set_strokeStyle_122f7f696ce9772c: function(arg0, arg1) {
            arg0.strokeStyle = arg1;
        },
        __wbg_set_strokeStyle_a5baa9565d8b6485: function(arg0, arg1, arg2) {
            arg0.strokeStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textAlign_8f846effafbae46d: function(arg0, arg1, arg2) {
            arg0.textAlign = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textBaseline_a9304886c3f7ea50: function(arg0, arg1, arg2) {
            arg0.textBaseline = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_width_c0fcaa2da53cd540: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_shaderSource_2bca0edc97475e95: function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        },
        __wbg_static_accessor_GLOBAL_8adb955bd33fac2f: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_ad356e0db91c7913: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_f207c857566db248: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_bb9f1ba69d61b386: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_strokeRect_2e20ce9870736fad: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.strokeRect(arg1, arg2, arg3, arg4);
        },
        __wbg_stroke_affa71c0888c6f31: function(arg0) {
            arg0.stroke();
        },
        __wbg_translate_d7de7bdfdbc1ee9d: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.translate(arg1, arg2);
        }, arguments); },
        __wbg_uniform2f_8fc2c40c50fd770c: function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        },
        __wbg_uniform3f_1f319f9f4d116e54: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniform3f(arg1, arg2, arg3, arg4);
        },
        __wbg_useProgram_5405b431988b837b: function(arg0, arg1) {
            arg0.useProgram(arg1);
        },
        __wbg_vertexAttribDivisor_99b2fd5affca539d: function(arg0, arg1, arg2) {
            arg0.vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_vertexAttribPointer_ea73fc4cc5b7d647: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_viewport_b60aceadb9166023: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_width_eebf2967f114717c: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
            const ret = getArrayF32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./rendero_bg.js": import0,
    };
}

const __wbindgen_enum_CanvasWindingRule = ["nonzero", "evenodd"];
const CanvasEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_canvasengine_free(ptr >>> 0, 1));
const FigmaBenchFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_figmabench_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getBigInt64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedBigInt64ArrayMemory0 = null;
function getBigInt64ArrayMemory0() {
    if (cachedBigInt64ArrayMemory0 === null || cachedBigInt64ArrayMemory0.byteLength === 0) {
        cachedBigInt64ArrayMemory0 = new BigInt64Array(wasm.memory.buffer);
    }
    return cachedBigInt64ArrayMemory0;
}

function getClampedArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ClampedArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

let cachedUint8ClampedArrayMemory0 = null;
function getUint8ClampedArrayMemory0() {
    if (cachedUint8ClampedArrayMemory0 === null || cachedUint8ClampedArrayMemory0.byteLength === 0) {
        cachedUint8ClampedArrayMemory0 = new Uint8ClampedArray(wasm.memory.buffer);
    }
    return cachedUint8ClampedArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedBigInt64ArrayMemory0 = null;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    cachedUint8ClampedArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('rendero_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
