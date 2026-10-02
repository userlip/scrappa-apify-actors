use google_trends_interest_scraper::{
    apify::put_terminal_status_message,
    config::Config,
    runtime::{actor_error_message, run_actor},
};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            std::process::exit(1);
        }
    };
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = run_actor(&client, &config).await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = put_terminal_status_message(&client, &config, &message).await {
            eprintln!("Warning: {status_error}");
        }
        std::process::exit(1);
    }
}
