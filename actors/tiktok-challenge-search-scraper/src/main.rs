use tiktok_challenge_search_scraper::{actor_error_message, run_actor, ActorClient, ActorConfig};

#[tokio::main]
async fn main() {
    let result = async {
        let config = ActorConfig::from_env()?;
        let client = ActorClient::new(config)?;
        run_actor(&client).await
    }
    .await;

    if let Err(error) = result {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        std::process::exit(1);
    }
}
