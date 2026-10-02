mod actor;
mod apify;
mod apify_retry;
mod doctor_details;
mod scrappa;
mod scrappa_retry;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    actor::run().await
}
