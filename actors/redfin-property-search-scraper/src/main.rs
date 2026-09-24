mod actor;
mod apify;
mod charging;
mod request_params;
mod response_utils;
mod scrappa;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match actor::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}
