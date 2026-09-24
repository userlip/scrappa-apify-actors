# Arbeitsagentur Jobs Scraper

Rust source for `arbeitsagentur-jobs-scraper`. See `.actor/README.md` for marketplace-facing documentation.

From this directory, run `cargo test --locked` to exercise input normalization, response mapping, Scrappa retries, Apify storage, and pay-per-event budget handling. Build the local actor image with `docker build -f .actor/Dockerfile -t arbeitsagentur-jobs-scraper .`.
