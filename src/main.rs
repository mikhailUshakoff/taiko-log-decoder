mod consensus_layer;
mod env_reader;
mod pacaya;
mod shasta;

use std::env;

fn usage() -> ! {
    eprintln!("Usage: taiko-log-decoder --shasta | --pacaya");
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let arg = env::args().nth(1).unwrap_or_else(|| usage());

    let result = match arg.as_str() {
        "--shasta" => shasta::run().await,
        "--pacaya" => pacaya::run().await,
        _ => usage(),
    };

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
