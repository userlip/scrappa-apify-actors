mod apify_client;
mod apify_retry;
mod app;
mod job;
mod pricing;
mod scrappa;
mod scrappa_retry;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    app::run_actor().await
}
