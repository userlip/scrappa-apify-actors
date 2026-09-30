# Kleinanzeigen Search Scraper for Product Research

The Kleinanzeigen Search Scraper for Product Research collects marketplace listings, prices, and item details from Kleinanzeigen. Provide a search phrase or a short list of phrases; the actor saves source fields such as `id`, `title`, `price`, and `price_numeric` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Kleinanzeigen. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | Listing ID returned for this result. |
| `title` | text | Title returned for this result. |
| `price` | text | Price returned for this result. |
| `price_numeric` | number | Price Numeric returned for this result. |
| `location` | text | Location returned for this result. |
| `url` | link | Listing URL returned for this result. |
| `image_url` | image | Image returned for this result. |
| `description` | text | Description returned for this result. |
| `has_shipping` | boolean | Shipping returned for this result. |
| `request_query` | text | Search returned for this result. |
| `request_location` | text | Request Location returned for this result. |
| `request_category` | text | Category returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_price_min` | number | Min Price returned for this result. |
| `request_price_max` | number | Max Price returned for this result. |
| `results_count` | number | Results Count returned for this result. |

## Use cases

- Collect marketplace listings, prices, and item details to research product availability and pricing.
- Compare item, seller, and listing details across a small search batch.
- Export marketplace records for catalog or resale analysis.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `searches` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "page": 1,
  "location": "Berlin",
  "searches": [
    {
      "query": "iphone",
      "location": "Berlin",
      "page": 1
    }
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "example-123",
  "title": "Example result",
  "price": "129.99",
  "price_numeric": 129.99,
  "location": "Example location",
  "url": "https://example.com/result/1",
  "image_url": "https://example.com/image.jpg"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Kleinanzeigen search text, for example iphone, fahrrad, sofa, or wohnung. |
| `page` | integer | No | One-based Kleinanzeigen search results page. Constraints: minimum 1; maximum 100. |
| `location` | string | No | Optional city, district, or location filter, for example Berlin. |
| `category` | string | No | Optional category slug or keyword, for example elektronik, auto, or moebel. |
| `price_min` | integer | No | Minimum listing price in EUR. Constraints: minimum 0. |
| `price_max` | integer | No | Maximum listing price in EUR. Constraints: minimum 0. |
| `searches` | array of object | No | Optional batch of up to 25 Kleinanzeigen searches to run in a single Actor run. When provided, top-level search fields are ignored. Constraints: maximum 25 items. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Kleinanzeigen. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~kleinanzeigen-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper for Buyers](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Vinted Item Details Scraper for Product Research](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper for Product Research](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper for Product Research](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper for Seller Research](https://apify.com/thescrappa/vinted-user-profile-scraper)
