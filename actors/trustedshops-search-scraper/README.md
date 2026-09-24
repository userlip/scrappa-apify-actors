# Trusted Shops Search Scraper

Apify Actor source for searching Trusted Shops shop profiles through Scrappa.

See [.actor/README.md](.actor/README.md) for marketplace-facing usage docs.

## Development

Run the actor-focused Rust tests with `cargo test --locked`. Build the Docker image from this directory with:

```sh
docker build -f .actor/Dockerfile -t trustedshops-search-scraper .
```
