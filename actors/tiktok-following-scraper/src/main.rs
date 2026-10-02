#[tokio::main]
async fn main() {
    if let Err(error) = tiktok_following_scraper::run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}
