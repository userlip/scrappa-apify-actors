# Geizhals Price History Scraper

Retrieve Geizhals price-history points for a product ID, including timestamps and recorded prices. Choose a lookback window and market to compare changes over time.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `timestamp_ms` | Integer | Unix timestamp in milliseconds for this price observation. |
| `timestamp` | Integer | Unix timestamp in seconds for this price observation. |
| `price` | Number | Product price recorded at this timestamp, in the selected Geizhals market currency. |
| `marker` | Integer | Geizhals chart marker describing the type of price-history point. |
| `input_product_id` | String | Product id submitted to retrieve this price point. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Price analysts can chart historical changes for a product catalog.
- Retail teams can review low and high points before updating price rules.
- Shoppers can compare recent price movement across multiple product IDs.

## How to use

1. Add one Geizhals product ID to `product_ids` for each history lookup.
2. Choose the market and number of days to include.
3. Set `maxResults` to cap the saved history points, then export the rows.

```json
{
  "product_ids": [
    {
      "product_id": 3103639
    }
  ],
  "loc": "de",
  "days": 7,
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `product_ids` | Array\<object\> | Yes | Geizhals product IDs whose price history you want to review. |
| `product_ids[].product_id` | integer | Yes per entry | Known Geizhals product id from product URLs such as ...-a2194110.html. |
| `product_ids[].days` | integer | No | History window in days. Defaults to 31. Maximum 3650. |
| `product_ids[].loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `days` | integer | No | History window in days. Defaults to 31. Maximum 3650. |
| `loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "timestamp_ms": 1790000000000,
  "timestamp": 1790086400,
  "price": 549,
  "marker": 8,
  "input_product_id": 3103639,
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~geizhals-price-history-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What does one result represent?

Each row is one recorded price point with its source timestamp and price value.

## Related Scrappa Actors

- [Geizhals Product & Offers Scraper](https://apify.com/thescrappa/geizhals-product-scraper)
- [Geizhals Price Comparison Search Scraper](https://apify.com/thescrappa/geizhals-search-scraper)
- [Billiger.de Product Offers Scraper](https://apify.com/thescrappa/billiger-offers-scraper)
