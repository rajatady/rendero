//! Exports render test results as raw pixel data for snapshot diffing.
//! Output format: one line per test:
//!   TEST:<name>:<width>:<height>:<hash>:<pixel_summary>
//! where pixel_summary = <non_transparent_count>/<total_pixels>
//!
//! With --dump flag, also writes raw RGBA bytes to .snapshots/<name>.raw

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dump = args.iter().any(|a| a == "--dump");
    let dump_dir = if dump {
        let dir = PathBuf::from(".snapshots");
        fs::create_dir_all(&dir).ok();
        Some(dir)
    } else {
        None
    };

    let (passed, failed, new) = rendero_renderer::verify::run_all_tests();

    // Also run each test individually to get pixel data
    let tests = rendero_renderer::verify::get_all_tests();
    for test in &tests {
        let (pixels, hash) = rendero_renderer::verify::render_test_pixels(test);
        let total = (test.width * test.height) as usize;
        let non_transparent = (0..total)
            .filter(|&i| pixels[i * 4 + 3] > 0)
            .count();

        println!(
            "TEST:{}:{}:{}:{}:{}/{}",
            test.name, test.width, test.height, hash, non_transparent, total
        );

        if let Some(ref dir) = dump_dir {
            let path = dir.join(format!("{}.raw", test.name));
            fs::write(&path, &pixels).ok();
        }
    }

    println!("SUMMARY:{},{},{}", passed, failed, new);
}
