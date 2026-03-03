/* tslint:disable */
/* eslint-disable */

export class CanvasEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a layer blur effect to a node.
     */
    add_blur(counter: number, client_id: number, radius: number): boolean;
    /**
     * Add a comment at world position (x, y). Returns the comment ID.
     */
    add_comment(x: number, y: number, text: string, author: string): number;
    /**
     * Add a drop shadow effect to a node.
     */
    add_drop_shadow(counter: number, client_id: number, r: number, g: number, b: number, a: number, ox: number, oy: number, blur: number, spread: number): boolean;
    /**
     * Add an ellipse. Returns node ID as [counter, client_id].
     */
    add_ellipse(name: string, x: number, y: number, width: number, height: number, r: number, g: number, b: number, a: number): Uint32Array;
    /**
     * Batch add multiple ellipses in one call. Format: [x, y, w, h, r, g, b, a] × N.
     * Skips CRDT ops, undo stack, and per-node scene updates for maximum throughput.
     * Returns the number of ellipses added.
     */
    add_ellipses_batch(data: Float32Array): number;
    /**
     * Add a frame.
     */
    add_frame(name: string, x: number, y: number, w: number, h: number, r: number, g: number, b: number, a: number): Uint32Array;
    /**
     * Add a rectangle with a linear gradient fill.
     * stop_positions and stop_colors are parallel arrays. Each color is [r, g, b, a].
     */
    add_gradient_rectangle(name: string, x: number, y: number, width: number, height: number, start_x: number, start_y: number, end_x: number, end_y: number, stop_positions: Float32Array, stop_colors: Float32Array): Uint32Array;
    /**
     * Add an image node from raw RGBA pixel data.
     * Returns node ID as [counter, client_id].
     */
    add_image(name: string, x: number, y: number, width: number, height: number, image_data: Uint8Array, image_width: number, image_height: number): Uint32Array;
    /**
     * Add a rectangle with an image fill (URL-based, loaded by renderer).
     * `path` is relative to /imports/ (e.g. "starbucks.png").
     * `scale_mode`: "fill", "fit", "tile", "stretch".
     */
    add_image_fill(name: string, x: number, y: number, width: number, height: number, path: string, scale_mode: string, opacity: number): Uint32Array;
    /**
     * Add an inner shadow effect to a node.
     */
    add_inner_shadow(counter: number, client_id: number, r: number, g: number, b: number, a: number, ox: number, oy: number, blur: number, spread: number): boolean;
    /**
     * Add a line from (x1,y1) to (x2,y2) with stroke color.
     */
    add_line(name: string, x1: number, y1: number, x2: number, y2: number, r: number, g: number, b: number, a: number, stroke_width: number): Uint32Array;
    /**
     * Append a solid fill to existing fills (for multiple fills per node).
     */
    add_node_fill(counter: number, client_id: number, r: number, g: number, b: number, a: number): boolean;
    /**
     * Append a linear gradient fill to existing fills.
     */
    add_node_linear_gradient(counter: number, client_id: number, start_x: number, start_y: number, end_x: number, end_y: number, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Append a radial gradient fill to existing fills.
     */
    add_node_radial_gradient(counter: number, client_id: number, center_x: number, center_y: number, radius: number, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Add a new page and return its index.
     */
    add_page(name: string): number;
    /**
     * Add a GPU-direct point cloud from packed Float32Array: [x, y, w, h, r, g, b, a] × N.
     * Point clouds bypass the document tree entirely — data goes straight to GPU.
     * Returns cloud ID.
     */
    add_point_cloud(gl: WebGL2RenderingContext, data: Float32Array): number;
    /**
     * Add a prototype link from source node to target node.
     * trigger: "click" | "hover" | "drag"
     * animation: "instant" | "dissolve" | "slide"
     */
    add_prototype_link(src_counter: number, src_client: number, dst_counter: number, dst_client: number, trigger: string, animation: string): boolean;
    /**
     * Add a rectangle. Returns node ID as [counter, client_id].
     */
    add_rectangle(name: string, x: number, y: number, width: number, height: number, r: number, g: number, b: number, a: number): Uint32Array;
    /**
     * Add a rounded rectangle. Returns node ID as [counter, client_id].
     */
    add_rounded_rect(name: string, x: number, y: number, width: number, height: number, r: number, g: number, b: number, a: number, radius: number): Uint32Array;
    /**
     * Add a star/polygon. `points` = number of outer points (3=triangle, 5=star, 6=hexagon).
     * `inner_ratio` = inner radius / outer radius (0.0..1.0). Use 1.0 for regular polygon.
     */
    add_star(name: string, x: number, y: number, width: number, height: number, r: number, g: number, b: number, a: number, points: number, inner_ratio: number): Uint32Array;
    /**
     * Add a text node. Returns node ID as [counter, client_id].
     */
    add_text(name: string, content: string, x: number, y: number, font_size: number, r: number, g: number, b: number, a: number): Uint32Array;
    /**
     * Add a vector shape from flat path data.
     * Format: each command is [type, ...args]
     *   0, x, y           = MoveTo
     *   1, x, y           = LineTo
     *   2, c1x, c1y, c2x, c2y, x, y = CubicTo
     *   3                 = Close
     * `width`/`height` = bounding box for hit-testing.
     */
    add_vector(name: string, x: number, y: number, width: number, height: number, r: number, g: number, b: number, a: number, path_data: Float32Array): Uint32Array;
    /**
     * Align selected nodes. direction: 0=left, 1=center-h, 2=right, 3=top, 4=center-v, 5=bottom
     */
    align_selected(direction: number): boolean;
    /**
     * Apply remote operations (JSON array of Operation).
     * Returns number of ops applied.
     */
    apply_remote_ops(json: string): number;
    /**
     * Combine selected nodes with a boolean operation.
     * Creates a BooleanOp parent, moves selected nodes under it.
     * op: 0=Union, 1=Subtract, 2=Intersect, 3=Exclude
     */
    boolean_op(op: number): boolean;
    /**
     * Bring selected nodes forward one step in z-order.
     */
    bring_forward(): boolean;
    /**
     * Bring selected nodes to front (top of z-order within their parent).
     */
    bring_to_front(): boolean;
    /**
     * Cancel creation mode.
     */
    cancel_creating(): void;
    /**
     * Clear insert parent — subsequent adds go to page root.
     */
    clear_insert_parent(): void;
    /**
     * Remove all point clouds and free GPU resources.
     */
    clear_point_clouds(gl: WebGL2RenderingContext): void;
    /**
     * Get comment count.
     */
    comment_count(): number;
    /**
     * Copy selected nodes to internal clipboard.
     */
    copy_selected(): number;
    /**
     * Create a component from selected nodes (wraps them like group, but NodeKind::Component).
     * Returns component node ID as [counter, client_id], or empty on failure.
     */
    create_component(): Uint32Array;
    /**
     * Create an instance of a component. Deep-clones the component's children.
     * Returns instance node ID as [counter, client_id], or empty on failure.
     */
    create_instance(comp_counter: number, comp_client_id: number): Uint32Array;
    /**
     * Get current page index.
     */
    current_page_index(): number;
    /**
     * Delete a comment by ID.
     */
    delete_comment(comment_id: number): boolean;
    /**
     * Delete all selected nodes.
     */
    delete_selected(): boolean;
    /**
     * Detach an instance: convert it to a plain Frame, keeping its children.
     * Returns true on success.
     */
    detach_instance(): boolean;
    /**
     * Distribute selected nodes evenly. direction: 0=horizontal, 1=vertical
     */
    distribute_selected(direction: number): boolean;
    /**
     * Number of items drawn in last render frame (for diagnostics).
     */
    drawn_count(): number;
    /**
     * Duplicate selected nodes in-place (copy + paste in one step).
     */
    duplicate_selected(): number;
    /**
     * Exit the currently entered group. Selects the group itself.
     */
    exit_group(): void;
    /**
     * Export the entire document as JSON for persistence.
     */
    export_document_json(): string;
    /**
     * Export the canvas at 1:1 scale without selection overlay.
     * Returns raw RGBA pixel data. JS converts to PNG via canvas.
     */
    export_pixels(width: number, height: number): Uint8Array;
    /**
     * Export the current page as SVG string.
     */
    export_svg(width: number, height: number): string;
    /**
     * Find nodes by name substring. Returns JSON array of {counter, client_id, name, info}.
     */
    find_nodes_by_name(query: string): string;
    /**
     * Find nodes that use a specific image key. Returns JSON array of {counter, client_id, name}.
     */
    find_nodes_with_image(image_key: string): string;
    /**
     * Flatten selected node to a vector path (Cmd+E).
     * Converts rectangles, ellipses, polygons, etc. to their path representation.
     * Returns true on success.
     */
    flatten_selected(): boolean;
    /**
     * Get all image assets on the current page.
     * Returns JSON array: [{type:"node"|"fill", key:string, name:string, counter:u64, client_id:u32}]
     * "node" = NodeKind::Image (raw pixels), "fill" = Paint::Image (referenced by path).
     */
    get_all_image_keys(): string;
    /**
     * Get current camera state as [cam_x, cam_y, zoom].
     */
    get_camera(): Float32Array;
    /**
     * Get all comments as JSON array.
     */
    get_comments(): string;
    /**
     * Get creation preview rectangle [x, y, w, h] in world coords during drag.
     * Returns empty vec if not currently dragging a creation.
     */
    get_creation_preview(): Float32Array;
    /**
     * Returns the entered group's counter and client_id, or (-1, -1) if none.
     */
    get_entered_group(): BigInt64Array;
    /**
     * Get image bytes extracted from a .fig ZIP by path.
     * Returns the raw PNG/JPEG bytes, or empty vec if not found.
     */
    get_imported_image(path: string): Uint8Array;
    /**
     * Get layer list as JSON array: [{"id":[counter,client_id],"name":"..."}]
     */
    get_layers(): string;
    /**
     * Get a range of layers as JSON: [{"id":[counter,client_id],"name":"..."}]
     * `start` is 0-based index, `count` is max items to return.
     */
    get_layers_range(start: number, count: number): string;
    /**
     * Returns the current marquee selection rectangle in world coords, or empty if not dragging.
     * Format: [min_x, min_y, max_x, max_y]. Used by TypeScript to draw the selection overlay.
     */
    get_marquee_rect(): Float32Array;
    /**
     * Get properties of the selected node as JSON.
     * Returns empty string if nothing is selected.
     */
    get_node_info(counter: number, client_id: number): string;
    /**
     * Get a node's name by ID. Returns empty string if not found.
     */
    get_node_name(counter: number, client_id: number): string;
    /**
     * Get a node's world-space bounding box: [x, y, width, height].
     * Accounts for all parent transforms (works at any nesting depth).
     */
    get_node_world_bounds(counter: number, client_id: number): Float32Array;
    /**
     * Get pages as JSON: [{"index":0,"name":"Page 1"},...]
     */
    get_pages(): string;
    /**
     * Get pending ops as JSON and clear the queue.
     */
    get_pending_ops(): string;
    /**
     * Get all prototype links as JSON array.
     */
    get_prototype_links(): string;
    /**
     * Get selected node IDs. Returns flat array: [counter0, client0, counter1, client1, ...].
     */
    get_selected(): Uint32Array;
    /**
     * Get current snap grid size.
     */
    get_snap_grid(): number;
    /**
     * Get text-on-arc parameters for a node. Returns [radius, start_angle, letter_spacing] or empty.
     */
    get_text_arc(counter: number, client_id: number): Float32Array;
    /**
     * Get layer tree as flat DFS list with depth info.
     * `expanded_ids` is comma-separated "counter:client" pairs for expanded nodes.
     * Returns JSON: [{"id":[c,cl],"name":"...","depth":N,"hasChildren":bool,"kind":"frame"|...}]
     * Only descends into expanded nodes. Supports virtualized rendering.
     */
    get_tree_layers(expanded_ids: string, start: number, count: number): Uint32Array;
    /**
     * Get vector network data for a vector node as JSON.
     * Returns vertices + segments representation (graph-based, not sequential paths).
     * This is the Figma vector network format: vertices share positions,
     * segments connect pairs of vertices with bezier tangent handles.
     */
    get_vector_network(counter: number, client_id: number): string;
    /**
     * Get image fills visible in the current viewport as JSON.
     * Returns: [[path, screenX, screenY, screenW, screenH, opacity], ...]
     * JS uses this to overlay HTMLImageElements after WASM renders the scene.
     */
    get_visible_image_fills(width: number, height: number): string;
    /**
     * Group selected nodes into a Frame.
     */
    group_selected(): boolean;
    /**
     * Handle explicit double-click from browser dblclick event.
     * Enters group or vector editing mode for the node under cursor.
     * This avoids timing-based double-click detection which can fail
     * when the browser event loop adds latency between mousedown events.
     */
    handle_double_click(sx: number, sy: number): boolean;
    /**
     * Import a document from JSON snapshot, replacing the current document.
     * Returns status JSON: {"ok":true,"pages":N,"nodes":N} or {"ok":false,"error":"..."}
     */
    import_document_json(json: string): string;
    /**
     * Import a .fig binary directly. No external tools needed.
     * Returns JSON: {"pages":N,"nodes":N,"images":[path,...],"errors":[...]}
     */
    import_fig_binary(bytes: Uint8Array): string;
    /**
     * Import a .fig file's JSON (from fig2json) into the document.
     * Returns JSON: {"pages": N, "nodes": N, "errors": [...]}
     */
    import_fig_json(json_str: string, image_base: string): string;
    /**
     * Import a single page from fig JSON (for large files).
     * JS should parse the full JSON, extract each page object, and stringify it individually.
     */
    import_fig_page_json(page_json: string, image_base: string): string;
    /**
     * Whether we're in creation mode (waiting for mousedown).
     */
    is_creating(): boolean;
    /**
     * Check if screen coords are in the rotation zone (outside resize handles).
     */
    is_rotation_zone(sx: number, sy: number): boolean;
    /**
     * Whether we're in vector point editing mode.
     */
    is_vector_editing(): boolean;
    /**
     * Total number of layers (children of root).
     */
    layer_count(): number;
    /**
     * Handle mouse down. Coordinates are SCREEN space.
     * shift=true adds/removes from selection instead of replacing.
     * Returns true if something was selected.
     */
    mouse_down(sx: number, sy: number, shift: boolean): boolean;
    /**
     * Handle mouse move (drag/resize). Coordinates are SCREEN space.
     */
    mouse_move(sx: number, sy: number): void;
    /**
     * Handle mouse up. Emits CRDT ops for any drag/resize that happened.
     */
    mouse_up(): void;
    /**
     * Check if a re-render is needed.
     */
    needs_render(): boolean;
    constructor(name: string, client_id: number);
    node_count(): number;
    /**
     * Get number of pages.
     */
    page_count(): number;
    /**
     * Stop panning.
     */
    pan_end(): void;
    /**
     * Continue panning.
     */
    pan_move(screen_x: number, screen_y: number): void;
    /**
     * Start panning (called on middle-click down or space+click).
     */
    pan_start(screen_x: number, screen_y: number): void;
    /**
     * Paste clipboard nodes offset by (10,10). Selects the pasted nodes.
     */
    paste(): number;
    /**
     * Cancel pen tool and discard the path.
     */
    pen_cancel(): void;
    /**
     * Finish pen path as closed path (click on first anchor).
     */
    pen_finish_closed(): void;
    /**
     * Finish pen path as open path (double-click or Enter).
     */
    pen_finish_open(): void;
    /**
     * Get pen path data for overlay rendering.
     * Returns JSON: { anchors: [{x,y,hox,hoy,hix,hiy}], cursor: {x,y}, closed: false }
     */
    pen_get_state(): string;
    /**
     * Is the pen tool currently active?
     */
    pen_is_active(): boolean;
    /**
     * Mouse down in pen mode (screen coords). Adds an anchor.
     * If clicking near the first anchor, closes the path.
     */
    pen_mouse_down(sx: number, sy: number): void;
    /**
     * Mouse drag in pen mode (screen coords). Creates curve handles.
     */
    pen_mouse_drag(sx: number, sy: number): void;
    /**
     * Mouse move in pen mode (for preview line).
     */
    pen_mouse_move(sx: number, sy: number): void;
    /**
     * Mouse up in pen mode.
     */
    pen_mouse_up(): void;
    /**
     * Enter pen drawing mode.
     */
    pen_start(): void;
    /**
     * Total number of points across all point clouds.
     */
    point_cloud_count(): number;
    /**
     * Get prototype link count.
     */
    prototype_link_count(): number;
    /**
     * Redo the last undone action. Returns true if something was redone.
     */
    redo(): boolean;
    /**
     * Remove auto-layout from a frame.
     */
    remove_auto_layout(counter: number, client_id: number): boolean;
    /**
     * Remove all strokes from a node.
     */
    remove_node_stroke(counter: number, client_id: number): boolean;
    /**
     * Remove all prototype links from a source node.
     */
    remove_prototype_links(counter: number, client_id: number): boolean;
    /**
     * Rename a page.
     */
    rename_page(index: number, name: string): boolean;
    /**
     * Raster render — returns raw RGBA pixel buffer. Used for PNG export and fallback.
     */
    render(width: number, height: number): Uint8Array;
    /**
     * Canvas 2D vector render — draws directly to a browser canvas context.
     * GPU-accelerated, no pixel buffer transfer.
     * `dpr` is the device pixel ratio for crisp Retina rendering.
     */
    render_canvas2d(ctx: CanvasRenderingContext2D, width: number, height: number, dpr: number): void;
    /**
     * WebGL2 instanced render — batches Rects and Ellipses into 2 GPU draw calls.
     * 10-50x faster than Canvas2D for data-dense scenes (10K+ visible shapes).
     */
    render_webgl(gl: WebGL2RenderingContext, width: number, height: number, dpr: number): void;
    /**
     * Resolve or unresolve a comment.
     */
    resolve_comment(comment_id: number, resolved: boolean): boolean;
    /**
     * Select all direct children of the current page root.
     */
    select_all(): void;
    /**
     * Select a node by ID (from layers panel click). Replaces current selection.
     */
    select_node(counter: number, client_id: number): void;
    /**
     * Send selected nodes backward one step in z-order.
     */
    send_backward(): boolean;
    /**
     * Send selected nodes to back (bottom of z-order within their parent).
     */
    send_to_back(): boolean;
    /**
     * Set auto-layout on a frame node.
     * direction: 0=Horizontal, 1=Vertical
     * After setting, compute_layout is called to position children.
     */
    set_auto_layout(counter: number, client_id: number, direction: number, spacing: number, pad_top: number, pad_right: number, pad_bottom: number, pad_left: number): boolean;
    /**
     * Set camera position and zoom directly.
     */
    set_camera(x: number, y: number, zoom: number): void;
    /**
     * Set dash pattern on a node's stroke.
     */
    set_dash_pattern(counter: number, client_id: number, dashes: Float32Array): boolean;
    /**
     * Set image fill on an existing node (replace all fills with an image fill).
     */
    set_image_fill(counter: number, client_id: number, path: string, scale_mode: string, opacity: number): boolean;
    /**
     * Set parent for subsequent add_* calls (children go inside this node).
     */
    set_insert_parent(counter: number, client_id: number): void;
    /**
     * Set letter spacing on a text node (in pixels).
     */
    set_letter_spacing(counter: number, client_id: number, spacing: number): boolean;
    /**
     * Set line height on a text node (in pixels, 0 = auto).
     */
    set_line_height(counter: number, client_id: number, height: number): boolean;
    /**
     * Set angular (conic) gradient fill on any node. Replaces existing fills.
     */
    set_node_angular_gradient(counter: number, client_id: number, center_x: number, center_y: number, start_angle: number, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Set blend mode on a node. mode: 0=Normal, 1=Multiply, 2=Screen, 3=Overlay,
     * 4=Darken, 5=Lighten, 6=ColorDodge, 7=ColorBurn, 8=HardLight, 9=SoftLight,
     * 10=Difference, 11=Exclusion, 12=Hue, 13=Saturation, 14=Color, 15=Luminosity.
     */
    set_node_blend_mode(counter: number, client_id: number, mode: number): boolean;
    /**
     * Set constraints on a node. h: 0=left, 1=right, 2=leftRight, 3=center, 4=scale
     * v: 0=top, 1=bottom, 2=topBottom, 3=center, 4=scale
     */
    set_node_constraints(counter: number, client_id: number, h: number, v: number): boolean;
    /**
     * Set corner radius on a rectangle or frame node.
     * If all four values are the same, uses uniform radius. Otherwise per-corner.
     */
    set_node_corner_radius(counter: number, client_id: number, tl: number, tr: number, br: number, bl: number): boolean;
    /**
     * Set node fill color (RGBA 0-1 range).
     * For text nodes, also updates the per-run text color.
     */
    set_node_fill(counter: number, client_id: number, r: number, g: number, b: number, a: number): boolean;
    /**
     * Set all fills on a node from a JSON array. Handles solid, gradient, and image fills.
     * JSON format: [{"type":"solid","r":255,"g":0,"b":0,"a":1.0}, {"type":"linear","startX":0,...,"stops":[...]}, ...]
     */
    set_node_fills_json(counter: number, client_id: number, fills_json: string): boolean;
    /**
     * Set font family on a text node (e.g. "Inter", "Roboto", "Poppins").
     */
    set_node_font_family(counter: number, client_id: number, family: string): boolean;
    /**
     * Set font size of a text node.
     */
    set_node_font_size(counter: number, client_id: number, size: number): boolean;
    /**
     * Set font weight on a text node (300=Light, 400=Regular, 500=Medium, 600=Semibold, 700=Bold).
     */
    set_node_font_weight(counter: number, client_id: number, weight: number): boolean;
    /**
     * Set linear gradient fill on any node. Replaces existing fills.
     * start/end are in 0..1 normalized coordinates (relative to node bounds).
     */
    set_node_linear_gradient(counter: number, client_id: number, start_x: number, start_y: number, end_x: number, end_y: number, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Set or unset the mask flag on a node.
     * When true, the node's shape clips all subsequent siblings until the parent ends.
     */
    set_node_mask(counter: number, client_id: number, is_mask: boolean): boolean;
    /**
     * Set node name.
     */
    set_node_name(counter: number, client_id: number, name: string): boolean;
    /**
     * Set opacity on a node (0.0 to 1.0).
     */
    set_node_opacity(counter: number, client_id: number, opacity: number): boolean;
    /**
     * Set node position from the properties panel.
     */
    set_node_position(counter: number, client_id: number, x: number, y: number): boolean;
    /**
     * Set radial gradient fill on any node. Replaces existing fills.
     * center is in 0..1 normalized coordinates. radius is 0..1 (1.0 = full extent).
     */
    set_node_radial_gradient(counter: number, client_id: number, center_x: number, center_y: number, radius: number, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Set node rotation in degrees. Preserves scale.
     */
    set_node_rotation(counter: number, client_id: number, degrees: number): boolean;
    /**
     * Set node size from the properties panel.
     */
    set_node_size(counter: number, client_id: number, w: number, h: number): boolean;
    /**
     * Set stroke on a node (color + weight). Replaces all existing strokes.
     */
    set_node_stroke(counter: number, client_id: number, r: number, g: number, b: number, a: number, weight: number): boolean;
    /**
     * Set the text content of a text node.
     */
    set_node_text(counter: number, client_id: number, text: string): boolean;
    /**
     * Set snap-to-grid size. 0 = disabled, typical values: 1, 4, 8, 16, 32.
     */
    set_snap_grid(size: number): void;
    /**
     * Set stroke alignment: "inside", "center", or "outside".
     */
    set_stroke_align(counter: number, client_id: number, align: string): boolean;
    /**
     * Set text horizontal alignment: "left", "center", "right".
     */
    set_text_align(counter: number, client_id: number, align: string): boolean;
    /**
     * Set text-on-arc parameters. radius=0 removes arc rendering.
     * start_angle is in radians (−PI/2 = top of circle, PI/2 = bottom).
     */
    set_text_arc(counter: number, client_id: number, radius: number, start_angle: number, letter_spacing: number): void;
    /**
     * Set text decoration: "none", "underline", or "strikethrough".
     */
    set_text_decoration(counter: number, client_id: number, decoration: string): boolean;
    /**
     * Set gradient fill on text (all runs). Type: "linear" or "radial".
     * For linear: extra = [start_x, start_y, end_x, end_y]. For radial: extra = [center_x, center_y, radius].
     */
    set_text_gradient_fill(counter: number, client_id: number, gradient_type: string, extra: Float32Array, stop_positions: Float32Array, stop_colors: Float32Array): boolean;
    /**
     * Set text vertical alignment: "top", "center", or "bottom".
     */
    set_text_vertical_align(counter: number, client_id: number, align: string): boolean;
    set_viewport(width: number, height: number): void;
    /**
     * Start shape creation mode. Next mousedown+drag will create the shape.
     * shape_type: "rect", "ellipse", "frame", "star", "text"
     */
    start_creating(shape_type: string): void;
    /**
     * Switch to a different page by index.
     */
    switch_page(index: number): boolean;
    /**
     * Toggle a node in/out of the selection (shift-click in layers panel).
     */
    toggle_select_node(counter: number, client_id: number): void;
    /**
     * Undo the last action. Returns true if something was undone.
     */
    undo(): boolean;
    /**
     * Ungroup: move children of selected group to its parent, remove the group.
     */
    ungroup_selected(): boolean;
    /**
     * Exit vector editing mode (from JS, e.g. Escape key).
     */
    vector_edit_exit(): void;
    /**
     * Get vector edit state as JSON for overlay rendering.
     * Returns: {"anchors":[{x,y,hox,hoy,hix,hiy}],"selected":N,"closed":bool,"tx":F,"ty":F}
     */
    vector_edit_get_state(): string;
    /**
     * Zoom centered on a screen point. delta > 0 zooms in, < 0 zooms out.
     */
    zoom(delta: number, screen_x: number, screen_y: number): void;
    /**
     * Zoom to fit all content on the current page.
     */
    zoom_to_fit(): boolean;
}

export class FigmaBench {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return a zero-copy Float32Array view into WASM linear memory.
     * SAFETY: The view is invalidated if WASM memory grows (e.g. new allocations).
     * Caller must use it immediately within the same JS turn.
     */
    data_view(): Float32Array;
    constructor(count: number, width: number, height: number);
    rect_count(): number;
    /**
     * Update all positions in WASM. Tight loop, no JS calls, no GC.
     */
    update(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_canvasengine_free: (a: number, b: number) => void;
    readonly canvasengine_add_blur: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_add_comment: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly canvasengine_add_drop_shadow: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => number;
    readonly canvasengine_add_ellipse: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly canvasengine_add_ellipses_batch: (a: number, b: number, c: number) => number;
    readonly canvasengine_add_frame: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly canvasengine_add_gradient_rectangle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number) => [number, number];
    readonly canvasengine_add_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly canvasengine_add_image_fill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number];
    readonly canvasengine_add_inner_shadow: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => number;
    readonly canvasengine_add_line: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number];
    readonly canvasengine_add_node_fill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly canvasengine_add_node_linear_gradient: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => number;
    readonly canvasengine_add_node_radial_gradient: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
    readonly canvasengine_add_page: (a: number, b: number, c: number) => number;
    readonly canvasengine_add_point_cloud: (a: number, b: any, c: number, d: number) => number;
    readonly canvasengine_add_prototype_link: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
    readonly canvasengine_add_rectangle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly canvasengine_add_rounded_rect: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number];
    readonly canvasengine_add_star: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => [number, number];
    readonly canvasengine_add_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number];
    readonly canvasengine_add_vector: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => [number, number];
    readonly canvasengine_align_selected: (a: number, b: number) => number;
    readonly canvasengine_apply_remote_ops: (a: number, b: number, c: number) => number;
    readonly canvasengine_boolean_op: (a: number, b: number) => number;
    readonly canvasengine_bring_forward: (a: number) => number;
    readonly canvasengine_bring_to_front: (a: number) => number;
    readonly canvasengine_cancel_creating: (a: number) => void;
    readonly canvasengine_clear_insert_parent: (a: number) => void;
    readonly canvasengine_clear_point_clouds: (a: number, b: any) => void;
    readonly canvasengine_comment_count: (a: number) => number;
    readonly canvasengine_copy_selected: (a: number) => number;
    readonly canvasengine_create_component: (a: number) => [number, number];
    readonly canvasengine_create_instance: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_current_page_index: (a: number) => number;
    readonly canvasengine_delete_comment: (a: number, b: number) => number;
    readonly canvasengine_delete_selected: (a: number) => number;
    readonly canvasengine_detach_instance: (a: number) => number;
    readonly canvasengine_distribute_selected: (a: number, b: number) => number;
    readonly canvasengine_drawn_count: (a: number) => number;
    readonly canvasengine_duplicate_selected: (a: number) => number;
    readonly canvasengine_exit_group: (a: number) => void;
    readonly canvasengine_export_document_json: (a: number) => [number, number];
    readonly canvasengine_export_pixels: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_export_svg: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_find_nodes_by_name: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_find_nodes_with_image: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_flatten_selected: (a: number) => number;
    readonly canvasengine_get_all_image_keys: (a: number) => [number, number];
    readonly canvasengine_get_camera: (a: number) => [number, number];
    readonly canvasengine_get_comments: (a: number) => [number, number];
    readonly canvasengine_get_creation_preview: (a: number) => [number, number];
    readonly canvasengine_get_entered_group: (a: number) => [number, number];
    readonly canvasengine_get_imported_image: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_layers: (a: number) => [number, number];
    readonly canvasengine_get_layers_range: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_marquee_rect: (a: number) => [number, number];
    readonly canvasengine_get_node_info: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_node_name: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_node_world_bounds: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_pages: (a: number) => [number, number];
    readonly canvasengine_get_pending_ops: (a: number) => [number, number];
    readonly canvasengine_get_prototype_links: (a: number) => [number, number];
    readonly canvasengine_get_selected: (a: number) => [number, number];
    readonly canvasengine_get_snap_grid: (a: number) => number;
    readonly canvasengine_get_text_arc: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_tree_layers: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly canvasengine_get_vector_network: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_get_visible_image_fills: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_group_selected: (a: number) => number;
    readonly canvasengine_handle_double_click: (a: number, b: number, c: number) => number;
    readonly canvasengine_import_document_json: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_import_fig_binary: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_import_fig_json: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly canvasengine_import_fig_page_json: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly canvasengine_is_creating: (a: number) => number;
    readonly canvasengine_is_rotation_zone: (a: number, b: number, c: number) => number;
    readonly canvasengine_is_vector_editing: (a: number) => number;
    readonly canvasengine_layer_count: (a: number) => number;
    readonly canvasengine_mouse_down: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_mouse_move: (a: number, b: number, c: number) => void;
    readonly canvasengine_mouse_up: (a: number) => void;
    readonly canvasengine_needs_render: (a: number) => number;
    readonly canvasengine_new: (a: number, b: number, c: number) => number;
    readonly canvasengine_node_count: (a: number) => number;
    readonly canvasengine_page_count: (a: number) => number;
    readonly canvasengine_pan_end: (a: number) => void;
    readonly canvasengine_pan_move: (a: number, b: number, c: number) => void;
    readonly canvasengine_pan_start: (a: number, b: number, c: number) => void;
    readonly canvasengine_paste: (a: number) => number;
    readonly canvasengine_pen_cancel: (a: number) => void;
    readonly canvasengine_pen_finish_closed: (a: number) => void;
    readonly canvasengine_pen_finish_open: (a: number) => void;
    readonly canvasengine_pen_get_state: (a: number) => [number, number];
    readonly canvasengine_pen_is_active: (a: number) => number;
    readonly canvasengine_pen_mouse_down: (a: number, b: number, c: number) => void;
    readonly canvasengine_pen_mouse_drag: (a: number, b: number, c: number) => void;
    readonly canvasengine_pen_mouse_move: (a: number, b: number, c: number) => void;
    readonly canvasengine_pen_mouse_up: (a: number) => void;
    readonly canvasengine_pen_start: (a: number) => void;
    readonly canvasengine_point_cloud_count: (a: number) => number;
    readonly canvasengine_prototype_link_count: (a: number) => number;
    readonly canvasengine_redo: (a: number) => number;
    readonly canvasengine_remove_auto_layout: (a: number, b: number, c: number) => number;
    readonly canvasengine_remove_node_stroke: (a: number, b: number, c: number) => number;
    readonly canvasengine_remove_prototype_links: (a: number, b: number, c: number) => number;
    readonly canvasengine_rename_page: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_render: (a: number, b: number, c: number) => [number, number];
    readonly canvasengine_render_canvas2d: (a: number, b: any, c: number, d: number, e: number) => void;
    readonly canvasengine_render_webgl: (a: number, b: any, c: number, d: number, e: number) => void;
    readonly canvasengine_resolve_comment: (a: number, b: number, c: number) => number;
    readonly canvasengine_select_all: (a: number) => void;
    readonly canvasengine_select_node: (a: number, b: number, c: number) => void;
    readonly canvasengine_send_backward: (a: number) => number;
    readonly canvasengine_send_to_back: (a: number) => number;
    readonly canvasengine_set_auto_layout: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
    readonly canvasengine_set_camera: (a: number, b: number, c: number, d: number) => void;
    readonly canvasengine_set_dash_pattern: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_image_fill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly canvasengine_set_insert_parent: (a: number, b: number, c: number) => void;
    readonly canvasengine_set_letter_spacing: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_line_height: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_angular_gradient: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
    readonly canvasengine_set_node_blend_mode: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_constraints: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_corner_radius: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly canvasengine_set_node_fill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly canvasengine_set_node_fills_json: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_font_family: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_font_size: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_font_weight: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_linear_gradient: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => number;
    readonly canvasengine_set_node_mask: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_name: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_opacity: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_position: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_radial_gradient: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
    readonly canvasengine_set_node_rotation: (a: number, b: number, c: number, d: number) => number;
    readonly canvasengine_set_node_size: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_node_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly canvasengine_set_node_text: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_snap_grid: (a: number, b: number) => void;
    readonly canvasengine_set_stroke_align: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_text_align: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_text_arc: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly canvasengine_set_text_decoration: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_text_gradient_fill: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => number;
    readonly canvasengine_set_text_vertical_align: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly canvasengine_set_viewport: (a: number, b: number, c: number) => void;
    readonly canvasengine_start_creating: (a: number, b: number, c: number) => void;
    readonly canvasengine_switch_page: (a: number, b: number) => number;
    readonly canvasengine_toggle_select_node: (a: number, b: number, c: number) => void;
    readonly canvasengine_undo: (a: number) => number;
    readonly canvasengine_ungroup_selected: (a: number) => number;
    readonly canvasengine_vector_edit_exit: (a: number) => void;
    readonly canvasengine_vector_edit_get_state: (a: number) => [number, number];
    readonly canvasengine_zoom: (a: number, b: number, c: number, d: number) => void;
    readonly canvasengine_zoom_to_fit: (a: number) => number;
    readonly __wbg_figmabench_free: (a: number, b: number) => void;
    readonly figmabench_data_view: (a: number) => any;
    readonly figmabench_new: (a: number, b: number, c: number) => number;
    readonly figmabench_rect_count: (a: number) => number;
    readonly figmabench_update: (a: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
