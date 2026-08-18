//! Dev-loop benches for GSV boxes (no Criterion crate — std Instant).
//!
//! `cargo bench --bench gsv_dev`

use std::time::Instant;

use gsv::boxes::xtask;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, n) in [("products_tsv", 8usize)] {
        let start = Instant::now();
        for _ in 0..n {
            match name {
                "products_tsv" => {
                    let _ = xtask::products_tsv(&root);
                }
                "disk_report" => {
                    let _ = xtask::disk_report(&root, false);
                }
                _ => {}
            }
        }
        let ns = start.elapsed().as_nanos() / n as u128;
        println!("gsv_dev {name}: median-ish {ns} ns/iter ({n} runs)");
    }
}
