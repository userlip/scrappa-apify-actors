#[tokio::main]
async fn main() {
    if let Err(error) = tiktok_followers_scraper::run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}
