# Vinted Item Details Scraper for Product Research

The Vinted Item Details Scraper for Product Research collects public record details and identifying fields from Vinted. Provide the fields listed below; the actor saves source fields such as `id`, `title`, `description`, and `price_amount` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Vinted. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | Item ID returned for this result. |
| `title` | text | Title returned for this result. |
| `description` | text | Description returned for this result. |
| `price_amount` | text | Price returned for this result. |
| `price_currency` | text | Currency returned for this result. |
| `total_item_price` | text | Total Price returned for this result. |
| `shipping_price` | text | Shipping returned for this result. |
| `brand_name` | text | Brand returned for this result. |
| `category_name` | text | Category returned for this result. |
| `size_name` | text | Size returned for this result. |
| `condition` | text | Condition returned for this result. |
| `availability` | text | Availability returned for this result. |
| `url` | link | Listing URL returned for this result. |
| `image_url` | image | Image returned for this result. |
| `seller_login` | text | Seller returned for this result. |
| `seller_feedback_reputation` | number | Seller Rating returned for this result. |
| `favourite_count` | number | Favorites returned for this result. |
| `view_count` | number | Views returned for this result. |
| `request_item_id` | text | Requested ID returned for this result. |
| `request_country` | text | Country returned for this result. |
| `request_index` | number | Request Index returned for this result. |
| `request_success` | boolean | Success returned for this result. |
| `error_message` | text | Error returned for this result. |

## Use cases

- Collect public record details and identifying fields to research product availability and pricing.
- Compare item, seller, and listing details across a small search batch.
- Export marketplace records for catalog or resale analysis.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `item_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "item_ids": [
    "10198495179"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "12345678901",
  "title": "Example cotton sweatshirt",
  "description": "Example public listing description.",
  "price_amount": "29.99",
  "price_currency": "EUR",
  "total_item_price": "29.99",
  "shipping_price": "4.99"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `item_id` | string | No | Single Vinted item ID. Use item_ids for batches discovered from Vinted Search. |
| `item_ids` | array of string | No | Batch of Vinted item IDs. One dataset row is written per processed ID. Maximum 50 IDs per run. Constraints: maximum 50 items. |
| `country` | string | No | Vinted country market. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Vinted. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~vinted-item-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper for Buyers](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper for Product Research](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Search Scraper for Product Research](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper for Product Research](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper for Seller Research](https://apify.com/thescrappa/vinted-user-profile-scraper)
