mod actor;
mod apify;
mod doctor_details;
mod scrappa;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    actor::run().await
}
