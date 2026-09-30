# Vinted User Profile Scraper for Seller Research

The Vinted User Profile Scraper for Seller Research collects public profile fields and account details from Vinted. Provide the fields listed below; the actor saves source fields such as `id`, `login`, `country_code`, and `city` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Vinted. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | User ID returned for this result. |
| `login` | text | Login returned for this result. |
| `country_code` | text | Country returned for this result. |
| `city` | text | City returned for this result. |
| `feedback_count` | number | Feedback returned for this result. |
| `feedback_reputation` | number | Reputation returned for this result. |
| `positive_feedback_count` | number | Positive returned for this result. |
| `neutral_feedback_count` | number | Neutral returned for this result. |
| `negative_feedback_count` | number | Negative returned for this result. |
| `bundle_discount_enabled` | boolean | Bundle Discounts returned for this result. |
| `bundle_discounts` | array | Bundle Tiers returned for this result. |
| `item_count` | number | Active Items returned for this result. |
| `total_items_count` | number | Total Items returned for this result. |
| `followers_count` | number | Followers returned for this result. |
| `following_count` | number | Following returned for this result. |
| `last_activity` | text | Last Activity returned for this result. |
| `is_email_verified` | boolean | Email Verified returned for this result. |
| `is_facebook_verified` | boolean | Facebook Verified returned for this result. |
| `is_google_verified` | boolean | Google Verified returned for this result. |
| `business` | boolean | Business returned for this result. |
| `is_on_holiday` | boolean | On Holiday returned for this result. |
| `is_account_banned` | boolean | Banned returned for this result. |
| `profile_url` | link | Profile URL returned for this result. |
| `request_user_id` | text | Requested ID returned for this result. |
| `request_country` | text | Requested Country returned for this result. |
| `request_index` | number | Request Index returned for this result. |
| `request_success` | boolean | Success returned for this result. |
| `scrappa_duration_ms` | number | Scrappa Duration (ms) returned for this result. |
| `scrappa_scraped_at` | date | Scraped At returned for this result. |

## Use cases

- Collect public profile fields and account details to research product availability and pricing.
- Compare item, seller, and listing details across a small search batch.
- Export marketplace records for catalog or resale analysis.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `user_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "user_ids": [
    "255914028"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "example-123",
  "login": "Example value",
  "country_code": "42",
  "city": "New York",
  "feedback_count": 42,
  "feedback_reputation": 42,
  "positive_feedback_count": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `user_id` | string | No | Single Vinted user ID. Use user_ids for batches; both fields can be combined. |
| `user_ids` | array/string | No | Batch of Vinted user IDs as an array or comma-separated string. The Actor validates numeric IDs and enforces a maximum of 100 unique IDs per run. |
| `country` | string | No | Vinted country market. Defaults to FR. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |

## Pricing

**Current live price:** $0.50 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Vinted. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~vinted-user-profile-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper for Buyers](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper for Product Research](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper for Product Research](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper for Product Research](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper for Product Research](https://apify.com/thescrappa/vinted-user-items-scraper)
