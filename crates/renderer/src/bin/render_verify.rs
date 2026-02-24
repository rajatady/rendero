//! Render verification binary.
//! Run: cargo run --bin render_verify
//! Output: RENDER: X passed, Y failed, Z new

fn main() {
    let (passed, failed, new) = rendero_renderer::verify::run_all_tests();
    if failed > 0 {
        println!("RENDER: FAIL — {} passed, {} failed, {} new", passed, failed, new);
        std::process::exit(1);
    } else {
        println!("RENDER: OK — {} passed, {} new", passed, new);
    }
}
