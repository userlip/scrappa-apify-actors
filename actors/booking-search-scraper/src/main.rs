#[tokio::main]
async fn main() {
    booking_search_scraper::run_from_env().await;
}
