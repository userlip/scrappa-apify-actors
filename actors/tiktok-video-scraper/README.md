# TikTok Video Details Scraper

Apify actor for Scrappa's `/api/tiktok/video` endpoint. It fetches TikTok video metadata, author details, engagement metrics, covers, music fields, and playback or download URLs. Scrappa performs the scraping; the actor accepts batches and writes one dataset item per processed URL.

## Local Development

```bash
cargo test --locked
cargo build --release --locked
docker build -f .actor/Dockerfile -t tiktok-video-scraper:local .
```

## Example Input

```json
{
  "urls": [
    "https://www.tiktok.com/@tiktok/video/7568510388342443294",
    "https://vm.tiktok.com/ZGeqDY4yL/"
  ],
  "hd": true
}
```

You can also provide `url` for single-URL API compatibility. `urls` is preferred because batching amortizes Apify run startup costs while Scrappa performs the scraping work.

## Output

Each requested TikTok URL is saved as one dataset item. Successful rows include the raw Scrappa video fields plus:

```json
{
  "request_url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "request_index": 1,
  "request_hd": true,
  "result_found": true,
  "processed_time": 1.23
}
```

If Scrappa returns no video data or an individual lookup fails, the actor pushes a row with `result_found: false`. Failed lookup rows include `error_message`; each row retains the original request order and uses a one-based `request_index`.

The actor reads input from the run's `INPUT` key-value-store record and writes results to the default dataset.

## Errors and Limits

Each Scrappa request has a 60-second deadline. Individual Scrappa HTTP errors, API error codes, malformed responses, and timeouts are saved as error rows; the actor continues with the next affordable request. Input, Apify pricing, and dataset-storage errors fail the run. Safe Apify input and pricing GET requests retry up to two times on network errors, HTTP 429, and server errors. Dataset POSTs are not retried to avoid duplicate rows.

The actor checks the run's pay-per-event pricing and existing charged events before saving results. It stops at the first URL that would exceed `maxTotalChargeUsd`; each dataset item is charged automatically as the default dataset-item event.

## Publication Pricing Gate

Before publishing this actor publicly, schedule paid Apify monetization as `PAY_PER_EVENT` on the default dataset-item event at `$0.0002/result` (`$0.20/1k results`), or the earliest Apify-allowed activation date if immediate pricing is blocked. Verify `pricingInfos` through the Apify API before public launch.
