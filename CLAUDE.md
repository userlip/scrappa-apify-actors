# Claude Instructions

## Apify Cost Control

Scraping must stay on Scrappa infrastructure. Apify actors in this repo should be thin marketplace wrappers around Scrappa API endpoints, not the place where heavy scraping work runs.

For actors that process one URL/entity per result, especially LinkedIn actors, always preserve and prefer batch input:

- Accept multiple URLs/entities in a single Apify run whenever the upstream Scrappa API supports it, or loop through the provided list inside one run when it does not.
- Push one dataset item per processed URL/entity so Apify monetization still charges per result.
- Do not design actors so normal customer usage requires one Apify run per LinkedIn URL/profile/company. That pattern caused very high Apify costs because run startup/runtime/storage overhead was paid for almost every single result.
- Keep Apify memory and timeout settings minimal for wrapper actors. The actor should mostly validate input, call `https://scrappa.co/api`, and write dataset items.
- Avoid unnecessary key-value store writes per item. Use dataset output as the primary result channel unless the actor has a clear compatibility reason to write `OUTPUT`.

Cost evidence from Apify analytics for TheScrappa through 2026-05-19:

- `linkedin-company-scraper`: 266,069 runs for 254,552 results, with $48.77 cost on $58.19 revenue.
- `linkedin-profile-scraper`: 141,503 runs for 139,180 results, with $30.03 cost on $28.60 revenue.
- `google-search-scraper`: 168,176 runs for 554,393 results, with $25.71 cost on $94.34 revenue.

The difference is run amortization. Google Search gets multiple results per Apify run, while LinkedIn has historically been near one run per result. Future implementations should reduce Apify run count per result by batching, while keeping the real scraping on Scrappa servers.
