# Geizhals Product & Offers Scraper

Look up Geizhals products by ID and retrieve the product record, merchant offers, category, price summary, and rating details. Add several IDs to inspect catalog items together.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Geizhals request returned a successful response. |
| `product_id` | Integer | Numeric identifier for this Geizhals product record. |
| `loc` | String | Geizhals locale code used to select the product page language and market. |
| `lang` | String | Language code of the returned Geizhals product record. |
| `product` | Object | Geizhals product details, including manufacturer, categories, price summary, offers, and product links. |
| `meta` | Object | Response metadata from Geizhals, such as request duration and endpoint family. |
| `product_name` | String | Full Geizhals product name, including model and capacity. |
| `manufacturer` | String | Manufacturer name associated with this Geizhals product. |
| `best_price` | Integer | Best price amount for this product record; currency follows the selected source market. |
| `offer_count` | Integer | Number of offer records reported for this product record. |
| `product_url` | String | Public Geizhals URL for the product record or linked resource. |
| `input_product_id` | String | Product id submitted to retrieve this product record. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Catalog managers can check manufacturer details and available merchant offers.
- Pricing analysts can compare current best prices with product rating summaries.
- Shopping teams can enrich product lists with Geizhals links and identifiers.

## How to use

1. Add one Geizhals product ID to `product_ids` per lookup.
2. Choose the market and language, then set how many offers and details to include.
3. Set `maxResults` and export one full product record per ID.

```json
{
  "product_ids": [
    {
      "product_id": "3103639"
    }
  ],
  "loc": "de",
  "lang": "de",
  "offers": 2,
  "merchant_details": false,
  "reviews": false,
  "videos": false,
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `product_ids` | Array\<object\> | Yes | Geizhals product IDs from product links or search results. |
| `product_ids[].product_id` | integer | Yes per entry | Known Geizhals product id from product URLs such as ...-a2194110.html or a search gzhid. |
| `product_ids[].loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `product_ids[].lang` | string | No | Response language: de or en. Defaults to de. |
| `product_ids[].offers` | integer | No | Maximum number of merchant offers to include \(1-100\). Defaults to 20. |
| `product_ids[].merchant_details` | boolean | No | Include merchant/shop details on offers. Defaults to true. |
| `product_ids[].reviews` | boolean | No | Include test reviews and review details. Defaults to true. |
| `product_ids[].videos` | boolean | No | Include product videos. Defaults to true. |
| `loc` | string | No | Market code: de, at, eu, pl, or uk. Defaults to de. |
| `lang` | string | No | Response language: de or en. Defaults to de. |
| `offers` | integer | No | Maximum number of merchant offers to include \(1-100\). Defaults to 20. |
| `merchant_details` | boolean | No | Include merchant/shop details on offers. Defaults to true. |
| `reviews` | boolean | No | Include test reviews and review details. Defaults to true. |
| `videos` | boolean | No | Include product videos. Defaults to true. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "product_id": 8000170,
  "loc": "Berlin",
  "lang": "de",
  "product": {
    "images": [
      "https://cdn.northstar.invalid/images/aurora-front.webp"
    ],
    "productvideo": [
      "Verified marketplace detail"
    ],
    "bpoffer_link": "Verified source detail",
    "category": [
      {
        "id": {
          "m": 8
        },
        "label": "Verified source detail"
      }
    ],
    "test_reviews": [
      {
        "ctime": 8,
        "status": "Published",
        "title": "Northstar Aurora 1 TB Smartphone",
        "logo_url": {
          "w": 8,
          "u": "Current listing detail",
          "h": 8
        },
        "flag": "verified",
        "position": 8,
        "abstract_status": "Published"
      }
    ],
    "average_test_reviews_metascore": 94,
    "manufacturer_name": "Northstar",
    "variant_id": 8000168,
    "product": "Northstar Aurora 1 TB Smartphone",
    "prices": {
      "best": 549,
      "avg": 12.8
    },
    "gtin": [
      "9345678901234"
    ],
    "urls": {
      "manufacturer": "Verified source detail",
      "rate": "Verified source detail",
      "pricehist": "€549.90",
      "reviews": "Verified source detail",
      "overview": "Verified source detail",
      "offers": "Verified source detail"
    },
    "deals": {
      "alltime": null,
      "change": null,
      "oldprice": null,
      "top_deal": null
    },
    "offer_count": 12,
    "description": [
      {
        "value": "USB-C interface",
        "prop": "USB-C interface"
      }
    ],
    "offers": [
      {
        "price": {
          "currency": "EUR",
          "ppu": null,
          "value": 12.8,
          "url": "https://geizhals.de/northstar-aurora-1-tb-a4450000.html",
          "legal": null,
          "date": "2026-09-18"
        },
        "pricing": {
          "loc_currency": "EUR",
          "loc_value": 12.8,
          "eur_value": 12.8,
          "orig_value": 12.8,
          "loc": "Berlin",
          "orig_currency": "EUR",
          "orig_loc": "Berlin"
        },
        "avl": {
          "best_code": 8,
          "descr": [
            {
              "text": "High quality and prompt delivery.",
              "branch": "Verified source detail"
            }
          ],
          "time": null
        },
        "shop": {
          "opening_hours": null,
          "url": "https://geizhals.de/northstar-aurora-1-tb-a4450000.html",
          "agb_link": "Verified source detail",
          "latitude": null,
          "multimerchant_id": 8000831,
          "id": 8000232,
          "longitude": null
        }
      }
    ],
    "gzhid": 8000554,
    "manufacturer_id": 8000714,
    "listed_since": "2026-09-18",
    "product_for_sort": "relevance",
    "variant_count": 28
  },
  "meta": {
    "duration_ms": 8,
    "attempts": 8
  },
  "product_name": "Northstar Aurora 1 TB Smartphone",
  "manufacturer": "Northstar",
  "best_price": 549,
  "offer_count": 12,
  "product_url": "Verified source detail",
  "input_product_id": "3103639",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~geizhals-product-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does each row contain merchant offers?

Yes. The product object includes the source offer list when offers are available; set the offer limit to control how many are requested.

## Related Scrappa Actors

- [Geizhals Price Comparison Search Scraper](https://apify.com/thescrappa/geizhals-search-scraper)
- [Geizhals Price History Scraper](https://apify.com/thescrappa/geizhals-price-history-scraper)
- [Billiger.de Product Offers Scraper](https://apify.com/thescrappa/billiger-offers-scraper)
