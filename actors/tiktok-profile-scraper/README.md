# TikTok Profile Scraper

Apify actor for Scrappa's `/api/tiktok/user/profile` endpoint.

## Build and test

```bash
cargo test --locked
docker build -f .actor/Dockerfile -t tiktok-profile-scraper:local .
```

The Rust actor reads `INPUT` from the run's default key-value store, calls Scrappa once with a 60-second deadline, appends a normalized profile to the default dataset, then stores the full Scrappa response in the `OUTPUT` key-value-store record. The API returns one profile per lookup, so the actor does not paginate.

On pay-per-event runs, writing a profile to the default dataset triggers Apify's `apify-default-dataset-item` synthetic charge. Before the write, the actor reads the run's pricing and charged-event counts and skips the dataset item if it would exceed `maxTotalChargeUsd`; it still saves the full API response to `OUTPUT`.

## Example Input

```json
{
  "profile": "@tiktok"
}
```

You can also provide a full profile URL, such as `https://www.tiktok.com/@tiktok`, or a numeric TikTok user ID in `profile`. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`.
