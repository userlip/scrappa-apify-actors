# Instagram Post Info

Retrieve detailed Instagram post data from a full post URL, shortcode, or the legacy `media_id` field. The actor calls Scrappa's Instagram Post API and returns engagement metrics, media URLs, author details, captions, and post content.

## Input

```json
{
  "url": "https://www.instagram.com/instagram/p/DdUYPr8Piav/"
}
```

The input form keeps the prefilled example URL. The prefill is not an API default, so a supplied `shortcode` or `media_id` still selects its own target. If `url` is a shortcode rather than a URL, it is treated as the shortcode input. When multiple fields are provided, a non-empty `url` has priority, followed by `shortcode`, then `media_id`.

## Availability and retries

If the single-post lookup is temporarily unavailable or requires Instagram login, a URL with an account username enables a fallback through Scrappa's recent user posts. The actor returns only a post with the exact requested shortcode. A missing match returns the original single-post error; errors from the fallback are surfaced directly. Shortcode-only input cannot use the account-feed fallback.

Each attempt has one shared 60-second deadline across both endpoints. Transient failures are retried after 5 and 15 seconds, for up to 200 seconds of request and wait time. Explicit `retryable: false` responses and ordinary authentication or validation failures are not retried. After a rate-limit response, an authentication-required response can be retried during cooldown.

## Output and pricing

The raw successful Scrappa response is written to the default dataset and the `OUTPUT` key-value-store record. The actor reads the current pay-per-event prices, charges, and `maxTotalChargeUsd` before publishing a dataset item. Apify charges the default dataset-item event only when that item is saved; failed lookups and fallback attempts do not publish or charge a result. If the remaining budget cannot cover the item, the dataset row is skipped and the successful response remains in `OUTPUT`.

The default run uses 128 MB and a 300-second timeout. Suggested pricing is $0.20 per 1,000 saved results.

## Development

The actor runtime is a native Rust binary. Run focused Rust tests with `cargo test --locked`. `npm test` uses a local Rust toolchain, a permitted Rust container, or installs pinned Rust 1.90 in the runner's temporary directory.
