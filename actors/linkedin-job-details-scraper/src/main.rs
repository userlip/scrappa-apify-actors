mod apify_client;
mod app;
mod job;
mod pricing;
mod scrappa;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    app::run_actor().await
}
