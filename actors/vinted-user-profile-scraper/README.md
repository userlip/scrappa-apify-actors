# Vinted User Profile Scraper

Research public Vinted seller identity, reputation, activity, and marketplace trust signals through Scrappa. This paid Actor accepts multiple seller IDs in one run and writes one dataset row per successfully resolved public profile.

Pricing is configured for pay-per-event usage with the `user-profile-result` event: **$0.0005 per successful profile**. Failed, private, banned, deleted, or incomplete profiles are logged and skipped without a charged profile row.

## Use cases

- Seller research and trust scoring
- Competitor and marketplace monitoring
- Reputation and activity analysis
- Finding business accounts, bundle discounts, and verification signals

For higher-volume workloads or direct API integration, use the [Scrappa API](https://scrappa.co).

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `user_id` | string | No | One Vinted user ID; retained for compatibility |
| `user_ids` | array or CSV string | No | Multiple Vinted user IDs. Singular and batch fields can be combined |
| `country` | string | No | Vinted market code. Defaults to `FR` |

The Actor trims and deduplicates IDs in input order, validates numeric IDs and country codes, and accepts at most 100 unique IDs per run. Requests use at most eight concurrent Scrappa calls with two 15-second attempts, keeping the maximum batch within the Actor's 600-second runtime. Supported countries are `FR`, `DE`, `ES`, `IT`, `NL`, `BE`, `AT`, `PL`, `CZ`, `LT`, `LU`, `SK`, `HU`, `RO`, `PT`, `SE`, `DK`, `FI`, and `US`.

### Batch example

```json
{
  "user_ids": ["255914028", "123456789"],
  "country": "DE"
}
```

The Actor API also accepts a comma-separated batch string in `user_ids`:

```json
{
  "user_ids": "255914028, 123456789",
  "country": "DE"
}
```

## Output

Each successful public profile is saved as exactly one dataset item. Raw public profile fields are preserved alongside stable normalized fields:

```json
{
  "id": 255914028,
  "login": "agranier",
  "country_code": "DE",
  "city": "Wiesbaden",
  "feedback_count": 46,
  "feedback_reputation": 0.98,
  "positive_feedback_count": 45,
  "neutral_feedback_count": 0,
  "negative_feedback_count": 1,
  "bundle_discount_enabled": true,
  "bundle_discounts": [
    { "minimal_item_count": 2, "fraction": "0.05" }
  ],
  "item_count": 7,
  "total_items_count": 19,
  "followers_count": 0,
  "following_count": 0,
  "last_activity": "2026-07-13T10:12:36+02:00",
  "is_email_verified": true,
  "is_facebook_verified": false,
  "is_google_verified": false,
  "business": false,
  "is_on_holiday": false,
  "profile_url": "https://www.vinted.de/member/255914028-agranier",
  "request_user_id": "255914028",
  "request_country": "DE",
  "request_index": 0,
  "request_success": true,
  "scrappa_duration_ms": 2451.07,
  "scrappa_scraped_at": "2026-07-13T09:49:21Z"
}
```

The dataset is the primary output channel. The Actor does not write per-profile key-value records or duplicate profile rows in `OUTPUT`. Scraping is performed by Scrappa's `vinted-user-profile` endpoint; this Actor does not access Vinted directly.

## Development

```bash
npm install
npm test
npm run typecheck
```
