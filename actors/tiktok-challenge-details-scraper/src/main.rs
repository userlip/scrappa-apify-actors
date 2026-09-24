use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match tiktok_challenge_details_scraper::run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}
