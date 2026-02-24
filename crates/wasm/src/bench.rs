//! Benchmark harness — purpose-built for canvas-engines-comparison.
//! Hybrid approach: WASM updates positions (fast), JS draws (no boundary crossing).
//! Returns a Float32Array view into WASM memory for zero-copy JS access.

use wasm_bindgen::prelude::*;
use js_sys::Float32Array;

#[wasm_bindgen]
pub struct FigmaBench {
    /// Flat interleaved layout: [x0, y0, size0, speed0, x1, y1, size1, speed1, ...]
    /// 4 floats per rect. Contiguous for SIMD-friendly iteration.
    data: Vec<f32>,
    count: usize,
    canvas_width: f32,
}

#[wasm_bindgen]
impl FigmaBench {
    #[wasm_bindgen(constructor)]
    pub fn new(count: u32, width: f32, height: f32) -> Self {
        let n = count as usize;
        let mut data = vec![0.0f32; n * 4];

        let mut seed: u32 = 12345;
        let mut rng = move || -> f32 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed as f32) / (u32::MAX as f32)
        };

        for i in 0..n {
            let base = i * 4;
            let s = (10.0 + rng() * 40.0).floor();
            data[base] = rng() * width;         // x
            data[base + 1] = (rng() * height).floor(); // y
            data[base + 2] = s;                  // size
            data[base + 3] = 1.0 + rng();       // speed
        }

        Self { data, count: n, canvas_width: width }
    }

    /// Update all positions in WASM. Tight loop, no JS calls, no GC.
    pub fn update(&mut self) {
        let cw = self.canvas_width;
        let data = &mut self.data;
        let n = self.count;
        for i in 0..n {
            let base = i * 4;
            let size = data[base + 2];
            let speed = data[base + 3];
            let mut x = data[base] - speed;
            if x < -size {
                x += cw + size;
            }
            data[base] = x;
        }
    }

    /// Return a zero-copy Float32Array view into WASM linear memory.
    /// SAFETY: The view is invalidated if WASM memory grows (e.g. new allocations).
    /// Caller must use it immediately within the same JS turn.
    pub fn data_view(&self) -> Float32Array {
        unsafe { Float32Array::view(&self.data) }
    }

    pub fn rect_count(&self) -> usize {
        self.count
    }
}
