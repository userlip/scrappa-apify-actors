#[tokio::main]
async fn main() {
    if let Err(error) = immobilienscout24_price_insights_scraper::run_from_env().await {
        eprintln!("Actor failed: {error}");
        std::process::exit(1);
    }
}
