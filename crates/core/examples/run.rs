//! `cargo run -p proforma-core --example run -- deal.yaml [report]`
//! prints the result JSON, or one report as CSV.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).expect("read deal");
    let out = match args.get(2) {
        Some(r) => proforma_core::api::csv(&text, r),
        None => proforma_core::api::run(&text),
    };
    match out {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
