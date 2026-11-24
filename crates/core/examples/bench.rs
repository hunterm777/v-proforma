//! `cargo run --release -p proforma-core --example bench -- deal.yaml`
//! times parse, engine, report and JSON for one deal (best of 20).

use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("deal path");
    let text = std::fs::read_to_string(path).expect("read deal");
    let mut best = [f64::MAX; 4];
    for _ in 0..20 {
        let t0 = Instant::now();
        let d = proforma_core::deal::parse(&text).unwrap();
        let t1 = Instant::now();
        let o = proforma_core::engine::run(&d).unwrap();
        let t2 = Instant::now();
        let out = proforma_core::report::output(&o);
        let t3 = Instant::now();
        let s = serde_json::to_string(&out).unwrap();
        let t4 = Instant::now();
        std::hint::black_box(s);
        for (b, x) in best.iter_mut().zip([t1 - t0, t2 - t1, t3 - t2, t4 - t3]) {
            *b = b.min(x.as_secs_f64() * 1e3);
        }
    }
    println!(
        "parse {:.2} ms, engine {:.2} ms, report {:.2} ms, json {:.2} ms",
        best[0], best[1], best[2], best[3]
    );
}
