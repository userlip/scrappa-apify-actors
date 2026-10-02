use kleinanzeigen_search_scraper::{actor_failure_message, run_actor, ApifyClient, Config};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };
    let http = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };

    match run_actor(&http, &config).await {
        Ok(output) => {
            if let Some(status_message) = output.status_message {
                let apify = ApifyClient::new(&http, &config);
                if let Err(error) = apify.set_terminal_status_message(&status_message).await {
                    eprintln!("Actor failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Err(error) => {
            let message = actor_failure_message(&error);
            eprintln!("Actor failed: {message}");
            let apify = ApifyClient::new(&http, &config);
            if let Err(status_error) = apify.set_terminal_status_message(&message).await {
                eprintln!("Could not set the terminal Actor status message: {status_error}");
            }
            std::process::exit(1);
        }
    }
}
