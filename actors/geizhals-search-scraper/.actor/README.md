# Geizhals Price Comparison Search Scraper

Search Geizhals by product keyword and collect manufacturer names, product IDs, GTINs, price summaries, and rating information. Choose a market and page through matching products.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `description` | Array\<Object\> | Product specification entries, each pairing a property name with its value. |
| `manufacturer_id` | Integer | Geizhals identifier for the product manufacturer. |
| `variant_count` | Integer | Number of variants grouped under the product result. |
| `rating_percent` | Integer | Percentage rating reported for the product by Geizhals. |
| `category` | Array\<Object\> | Geizhals category records associated with the product. |
| `bpoffer_link` | String | Geizhals link to the best-price offer for this product. |
| `rating_stars` | Integer | Star rating displayed for the product. |
| `rating_comments` | Integer | Number of rating comments associated with the product. |
| `gtin` | Array\<String\> | Global Trade Item Number values identifying the product. |
| `urls` | Object | Geizhals product links for overview, offers, reviews, and price history. |
| `gzhid` | Integer | Geizhals product identifier. |
| `listed_since` | String | Date when the product first appeared in the Geizhals catalog. |
| `product_for_sort` | String | Product label used by Geizhals when sorting search results. |
| `bestprices` | Object | Lowest, highest, first, and most recent prices reported for the product. |
| `images` | Array\<String\> or null | Product image URLs returned by Geizhals. |
| `asins` | Array\<String\> | Amazon Standard Identification Numbers linked to the product. |
| `test_reviews` | Array\<Object\> | Product test review records with source, title, and score details. |
| `rating_count` | Integer | Number of Geizhals user ratings for the product. |
| `average_test_reviews_metascore` | Integer | Average metascore across product tests. |
| `product` | String | Product name as displayed in Geizhals search results. |
| `variant_id` | Integer | Geizhals identifier for the selected product variant. |
| `manufacturer_name` | String | Manufacturer name displayed for the product. |
| `input_query` | String | Query submitted to retrieve this product result. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Shopping researchers can compare product variants and price ranges across Geizhals markets.
- Retail teams can track manufacturers, GTINs, and product coverage by keyword.
- Buyers can export product IDs and links for a shortlist of hardware or electronics.

## How to use

1. Add one product keyword to `queries` for each search.
2. Choose a market and language, then optionally set category or manufacturer filters.
3. Set `maxPages` and `maxResults` before exporting product matches.

```json
{
  "queries": [
    {
      "query": "rtx 4070"
    }
  ],
  "loc": "de",
  "lang": "de",
  "page": 1,
  "pagesize": 5,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Product keywords to search across Geizhals markets. |
| `queries[].query` | string | Yes per entry | Search keyword, e.g. thinkpad. |
| `queries[].loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `queries[].lang` | string | No | Response language: de or en. Defaults to de. |
| `queries[].page` | integer | No | 1-based page number. Defaults to 1. |
| `queries[].pagesize` | integer | No | Results per page, between 1 and 1000. Defaults to 10. |
| `queries[].sort` | string | No | Optional sort key as used by Geizhals \(e.g. price\). Defaults to relevance. |
| `queries[].category` | string | No | Optional Geizhals category id to constrain results. |
| `queries[].manufacturer` | string | No | Optional manufacturer id to constrain results. |
| `loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `lang` | string | No | Response language: de or en. Defaults to de. |
| `page` | integer | No | 1-based page number. Defaults to 1. |
| `pagesize` | integer | No | Results per page, between 1 and 1000. Defaults to 10. |
| `sort` | string | No | Optional sort key as used by Geizhals \(e.g. price\). Defaults to relevance. |
| `category` | string | No | Optional Geizhals category id to constrain results. |
| `manufacturer` | string | No | Optional manufacturer id to constrain results. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "description": [
    {
      "prop": "USB-C interface",
      "value": "USB-C interface"
    }
  ],
  "manufacturer_id": 8000694,
  "variant_count": 28,
  "rating_percent": 94,
  "category": [
    {
      "label": "Verified source detail",
      "id": {
        "m": 8
      }
    }
  ],
  "bpoffer_link": "Verified source detail",
  "rating_stars": 94,
  "rating_comments": 94,
  "gtin": [
    "9345678901234"
  ],
  "urls": {
    "overview": "Verified source detail",
    "reviews": "Verified source detail",
    "pricehist": "€549.90",
    "rate": "Verified source detail",
    "manufacturer": "Verified source detail",
    "offers": "Verified source detail"
  },
  "gzhid": 8000544,
  "listed_since": "2026-09-18",
  "product_for_sort": "relevance",
  "bestprices": {
    "first": 8,
    "min": 549,
    "last": 8,
    "max": 549
  },
  "images": [
    "https://cdn.northstar.invalid/images/aurora-front.webp"
  ],
  "asins": [
    "B0N0RTHSTAR"
  ],
  "test_reviews": [
    {
      "ctime": 8,
      "title": "Northstar Aurora 1 TB Smartphone",
      "status": "Published",
      "position": 8,
      "flag": "verified",
      "logo_url": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "url": "https://geizhals.de/northstar-aurora-1-tb-a4450000.html",
      "abstract_status": "Published",
      "abstract": "Customers praise the clear product details and responsive service."
    }
  ],
  "rating_count": 94,
  "average_test_reviews_metascore": 94,
  "product": "Northstar Aurora 1 TB Smartphone",
  "variant_id": 8000177,
  "manufacturer_name": "Northstar",
  "input_query": "rtx 4070",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~geizhals-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which markets can I search?

The source accepts de, at, eu, pl, and uk market codes. Results and currency presentation follow the selected market.

## Related Scrappa Actors

- [Geizhals Product & Offers Scraper](https://apify.com/thescrappa/geizhals-product-scraper)
- [Geizhals Price History Scraper](https://apify.com/thescrappa/geizhals-price-history-scraper)
- [Billiger.de Price Comparison Search Scraper](https://apify.com/thescrappa/billiger-search-scraper)
