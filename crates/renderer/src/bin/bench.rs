use rendero_renderer::verify::bench_render;

fn main() {
    let counts = [100, 500, 1000, 5000, 10000, 50000, 80000];
    let viewport = 2000;

    println!("BENCH:objects,time_us,items_rendered");
    for &n in &counts {
        let (time_us, items) = bench_render(n, viewport);
        println!("BENCH:{},{},{}", n, time_us, items);
    }
}
