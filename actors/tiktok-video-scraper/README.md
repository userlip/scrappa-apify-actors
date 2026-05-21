# TikTok Video Details Scraper

Apify actor for Scrappa's `/api/tiktok/video` endpoint.

Use it to fetch TikTok video metadata, author details, engagement metrics, covers, music fields, and playback or download URLs. The actor accepts multiple URLs in one run and pushes one dataset item per requested URL.

## Local Development

```bash
npm install
npm test
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
  "request_hd": true,
  "result_found": true,
  "processed_time": 1.23
}
```

If Scrappa returns no video data for a requested URL, the actor still pushes a row with `result_found: false` so paid usage and batch accounting stay aligned to requested URLs.

## Publication Pricing Gate

Before publishing this actor publicly, schedule paid Apify monetization as `PAY_PER_EVENT` on the default dataset-item event at `$0.0002/result` (`$0.20/1k results`), or the earliest Apify-allowed activation date if immediate pricing is blocked. Verify `pricingInfos` through the Apify API before public launch.
