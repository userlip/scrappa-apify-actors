# Google Shopping Scraper

Collect Shopping offers for multiple phrases. Results include product names, merchants, prices, ratings, review totals, categories, and thumbnails.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Position of the offer in Google Shopping results, starting at 1. |
| `category` | String | Product category assigned to the shopping offer. |
| `title` | String | Product name displayed for this Google Shopping offer. |
| `source` | String | Merchant name selling the product. |
| `thumbnail` | String | Product image preview URL supplied with the offer. |
| `price` | String | Displayed offer price and currency when provided by the merchant. |
| `rating` | Number | Average customer rating shown for the product. |
| `reviews` | Integer | Number of customer reviews shown for the product. |
| `input_q` | String | Product search phrase submitted to Google Shopping. |
| `scraped_at` | String | UTC date and time when this shopping offer was collected. |

## Use cases

- Retail teams can compare merchant offers and displayed prices.
- E-commerce analysts can track ratings and review counts across products.
- Category managers can collect product links for assortment reviews.

## How to use

1. Add one product phrase to `queries` for each Shopping search.
2. Use `maxPages` to bound collection and `maxResults` to cap offers.
3. Compare merchant, price, rating, and review fields.

```json
{
  "queries": [
    {
      "q": "running shoes"
    }
  ],
  "maxResults": 20,
  "maxPages": 2
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Product phrases to search in Google Shopping. |
| `queries[].q` | string | Yes per entry | Product search query. |
| `queries[].gl` | string | No | Two-letter country code. Default: us. |
| `queries[].hl` | string | No | Two-letter interface language. Default: en. |
| `queries[].location` | string | No | Location name encoded as a best-effort canonical UULE. Cannot be used with uule; provide uule directly for exact targeting. |
| `queries[].uule` | string | No | Encoded Google location. Cannot be used with location. |
| `queries[].start` | integer | No | Zero-based pagination offset. Default: 0. |
| `queries[].device` | string | No | desktop, mobile, or tablet. Default: desktop. |
| `queries[].small_business` | boolean | No | Only return small-business results. Cannot be used with shoprs. |
| `queries[].shoprs` | string | No | Opaque Google Shopping filter token. Cannot be used with small\_business. |
| `gl` | string | No | Two-letter country code. Default: us. |
| `hl` | string | No | Two-letter interface language. Default: en. |
| `location` | string | No | Location name encoded as a best-effort canonical UULE. Cannot be used with uule; provide uule directly for exact targeting. |
| `uule` | string | No | Encoded Google location. Cannot be used with location. |
| `start` | integer | No | Zero-based pagination offset. Default: 0. |
| `device` | string | No | desktop, mobile, or tablet. Default: desktop. |
| `small_business` | boolean | No | Only return small-business results. Cannot be used with shoprs. |
| `shoprs` | string | No | Opaque Google Shopping filter token. Cannot be used with small\_business. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "position": 1,
  "category": "Electronics",
  "title": "Soundcore Space One Wireless Headphones",
  "source": "Soundcore Store",
  "thumbnail": "https://cdn.soundcore.com/products/space-one/black-front.jpg",
  "price": "$99.99",
  "rating": 4.6,
  "reviews": 1842,
  "input_q": "running shoes",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-shopping-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are prices normalized to one currency?

The Actor returns source price text. Use the displayed market and currency context when comparing offers.

## Related Scrappa Actors

- [Google Lens Visual Search Scraper](https://apify.com/thescrappa/google-lens-scraper)
- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
