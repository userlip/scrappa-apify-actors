# YouTube API Batch Videos

Fetch details for multiple YouTube videos in one run. The Actor accepts comma-separated YouTube video IDs and saves each returned video as an item in the default Apify dataset.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ids` | string | Yes | Comma-separated YouTube video IDs. The upstream API supports up to 50 IDs per request. |

## Example Input

```json
{
  "ids": "7eul_Vt6SZY,6QQQKJJBJOY"
}
```

## Output

One dataset item is stored per video returned by the API. Fields depend on the current Scrappa YouTube response and typically include video identifiers, title, channel metadata, thumbnails, duration, view counts, and publish metadata.

The prefilled input fetches two public videos for Apify's daily QA. Transient upstream errors (including 504 Gateway Timeout) and connection failures are retried up to three attempts, with a 60-second limit per request. Other 4xx responses, invalid input, and successful responses without a `videos` array fail immediately. The Actor has a five-minute overall timeout.

## Runtime

The Rust binary reads the `INPUT` record from the default key-value store and posts the returned video array to the default dataset. Apify creates one dataset item per array element. It uses `ACTOR_DEFAULT_KEY_VALUE_STORE_ID`, `ACTOR_DEFAULT_DATASET_ID`, `ACTOR_INPUT_KEY` (defaults to `INPUT`), and `APIFY_TOKEN` (Bearer authentication). Storage and upstream errors are reported by a nonzero process exit; failed writes are not treated as success.

`APIFY_API_PUBLIC_BASE_URL` may override `https://api.apify.com`, and `SCRAPPA_API_BASE_URL` may override `https://ytapi.scrappa.co` for local mock endpoints. The Rust tests exercise both services through loopback HTTP mocks and use only a dummy token, so no Apify credentials or live API calls are needed: run `cargo test --locked` from this directory. These test-only mock settings do not alter cloud behavior.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
