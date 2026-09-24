# Google Translate Scraper

Scrappa-powered Rust Apify Actor for batch Google Translate results. See `.actor/README.md` for the Apify Store listing content, input example, and output shape.

Run the focused actor tests with `cargo test --locked`. Build the local image from this directory with `docker build -f .actor/Dockerfile -t google-translate-scraper:local .`, then smoke it against local API doubles with `python3 test/local_image_smoke.py --image google-translate-scraper:local`.
