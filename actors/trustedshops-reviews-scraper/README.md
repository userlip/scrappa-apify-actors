# Trusted Shops Reviews Scraper

Rust Apify Actor for extracting Trusted Shops review records through Scrappa.

The Actor accepts the input fields documented in [.actor/README.md](.actor/README.md), writes one dataset item per review, and saves its page summary to the default key-value store `OUTPUT` record.

Run the actor-local Rust tests with `cargo test --locked` from this directory. The Docker build runs the same focused tests before compiling the production image.
