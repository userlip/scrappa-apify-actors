# Billiger.de Product Offers Scraper

Collect merchant offers for a Billiger.de product, including shop names, prices, shipping costs, stock text, and seller ratings. Batch product IDs to compare several listings together.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `shop` | String | Merchant name selling the product. |
| `shop_tags` | Array\<String\> | Labels describing the merchant or its offer. |
| `mp_seller_name` | String or null | Marketplace seller name when the offer comes from a third-party seller. |
| `shop_group_size` | String or null | Number of offers grouped for this shop, when reported. |
| `price_per_unit` | String | Offer price per unit in the source currency and format. |
| `availability_code` | String | Billiger.de availability code for this offer. |
| `brand_id` | Integer | Billiger.de identifier for the product brand. |
| `brand` | String | Brand name of the product in this offer. |
| `condition` | String | Condition of the offered item, such as new or refurbished. |
| `shop_id` | Integer | Billiger.de identifier for the merchant. |
| `category_id` | Integer | Billiger.de category identifier for the offered product. |
| `category` | String | Product category assigned by Billiger.de. |
| `category_tags` | Array\<String\> | Category labels associated with the offer. |
| `energy_tyre_labels` | Array\<Object\> | Tyre efficiency label records attached to the product. |
| `starting_prizes` | Boolean | True when the offer includes a starting or promotional price. |
| `shop_logo_url` | String | Merchant logo image URL. |
| `shipping_costs` | Integer | Delivery cost for this merchant offer, in the source currency. |
| `shop_threshold_free_shipping` | Integer or null | Minimum order amount for free shipping, when reported. |
| `type` | String | Billiger.de result type for this offer. |
| `price` | Integer | Advertised merchant offer price, in the source currency. |
| `promo_text` | String or null | Promotion text attached to the offer, when available. |
| `description` | String | Offer or product description shown by the merchant. |
| `old_price` | Integer or null | Previous offer price before a discount, in the source currency. |
| `availability_text` | String | Stock or delivery availability displayed for the offer. |
| `mp_feedback_url` | String or null | Marketplace seller feedback page URL, when available. |
| `mp_seller_userreview_count` | String or null | Number of marketplace seller reviews, when reported. |
| `shop_userreview_count` | Integer | Number of customer reviews for the shop. |
| `offer_id` | Integer | Billiger.de identifier for this merchant offer. |
| `voucher_text` | String or null | Voucher or coupon information attached to the offer. |
| `listed_price` | Integer | List price displayed before discount, in the source currency. |
| `total_price` | Integer | Offer price including delivery, in the source currency. |
| `mobile_optimized` | Boolean | True when the merchant offer page is marked as mobile optimized. |
| `name` | String | Product name associated with the merchant offer. |
| `mp_seller_userreview_rating` | Number or null | Marketplace seller rating; scale follows the source. |
| `shop_userreview_rating` | Number | Average customer rating for the shop; scale follows the source. |
| `shop_userreview_rating_combined` | Number | Combined shop rating across available review sources. |
| `shop_userreview_count_combined` | Integer | Combined number of shop reviews across available sources. |
| `payment_methods` | Array\<Object\> | Payment methods accepted for this offer. |
| `shipping_methods` | Array\<Object\> | Shipping options available for this offer. |
| `image_url_large` | String | Large product image URL associated with the offer. |
| `product_id` | Integer | Billiger.de product identifier for the item being offered. |
| `image_url` | String | Product image URL associated with the offer. |
| `has_image` | Boolean | True when an image is available for this offer. |
| `shop_data` | String or null | Additional merchant details supplied by Billiger.de. |
| `input_product_id` | String | Product id submitted to retrieve this shop offer. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Pricing teams can compare current merchant prices and shipping costs.
- Online retailers can track seller availability and ratings for competing products.
- Shoppers can export offer links and sort merchants before making a purchase.

## How to use

1. Add a Billiger.de product ID to `product_ids` for each lookup.
2. Choose a sort order and optional offer conditions.
3. Set page and result limits, then export one row per shop offer.

```json
{
  "product_ids": [
    {
      "product_id": 4612031183
    }
  ],
  "page": 1,
  "page_size": 5,
  "all": false,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `product_ids` | Array\<object\> | Yes | Billiger.de product IDs whose shop offers you want to compare. |
| `product_ids[].product_id` | integer | Yes per entry | Canonical Billiger product identifier. |
| `product_ids[].page` | integer | No | 1-based page number. Defaults to 1. |
| `product_ids[].page_size` | integer | No | Offers per page. Defaults to 20, maximum 50. |
| `product_ids[].sort` | string | No | relevance, price, total\_price, shop\_userreview\_rating, shop\_userreview\_count, or shop\_rating alias. |
| `product_ids[].direction` | string | No | Sort direction: asc or desc. |
| `product_ids[].group_by_shop` | boolean | No | Must remain false so offer pagination stays item-level and exhaustive. |
| `product_ids[].all` | boolean | No | Set true to exhaust every expected offer page. |
| `product_ids[].offer_conditions` | string | No | Numeric condition IDs such as 1 or 1,2,3. |
| `page` | integer | No | 1-based page number. Defaults to 1. |
| `page_size` | integer | No | Offers per page. Defaults to 20, maximum 50. |
| `sort` | string | No | relevance, price, total\_price, shop\_userreview\_rating, shop\_userreview\_count, or shop\_rating alias. |
| `direction` | string | No | Sort direction: asc or desc. |
| `group_by_shop` | boolean | No | Must remain false so offer pagination stays item-level and exhaustive. |
| `all` | boolean | No | Set true to exhaust every expected offer page. |
| `offer_conditions` | string | No | Numeric condition IDs such as 1 or 1,2,3. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "shop": "Northstar Outdoor Supply",
  "shop_tags": [
    "outdoor specialist"
  ],
  "mp_seller_name": "Morgenrot Handelskontor",
  "shop_group_size": null,
  "price_per_unit": "€549.90",
  "availability_code": "AVAILABLE",
  "brand_id": 8000825,
  "brand": "Northstar",
  "condition": "New",
  "shop_id": 8000751,
  "category_id": 8000272,
  "category": "Consumer electronics",
  "category_tags": [
    "Consumer electronics"
  ],
  "energy_tyre_labels": [
    {
      "icon": "primary-image",
      "desc": "Main listing image and product details."
    }
  ],
  "starting_prizes": true,
  "shop_logo_url": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "shipping_costs": 4,
  "shop_threshold_free_shipping": 4,
  "type": "Product",
  "price": 549,
  "promo_text": "High quality and prompt delivery.",
  "description": "Compact design with dependable performance and a two-year warranty.",
  "old_price": 629,
  "availability_text": "In stock",
  "mp_feedback_url": "https://www.billiger.de/products/northstar-aurora-1-tb",
  "mp_seller_userreview_count": null,
  "shop_userreview_count": 347,
  "offer_id": 8000857,
  "voucher_text": "High quality and prompt delivery.",
  "listed_price": 629,
  "total_price": 584,
  "mobile_optimized": true,
  "name": "Northstar Aurora 1 TB Smartphone",
  "mp_seller_userreview_rating": 4.7,
  "shop_userreview_rating": 4.7,
  "shop_userreview_rating_combined": 4.7,
  "shop_userreview_count_combined": 347,
  "payment_methods": [
    {
      "id": 8000242,
      "sort_key": 8,
      "description": "Compact design with dependable performance and a two-year warranty."
    }
  ],
  "shipping_methods": [
    {
      "id": 8000243,
      "sort_key": 8,
      "description": "Compact design with dependable performance and a two-year warranty."
    }
  ],
  "image_url_large": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "product_id": 8000209,
  "image_url": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "has_image": true,
  "shop_data": null,
  "input_product_id": 4612031183,
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~billiger-offers-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What does one result represent?

Each dataset row is one shop offer returned for a product. A product with several merchants can produce several rows.

## Related Scrappa Actors

- [Billiger.de Price Comparison Search Scraper](https://apify.com/thescrappa/billiger-search-scraper)
- [Geizhals Product & Offers Scraper](https://apify.com/thescrappa/geizhals-product-scraper)
- [Trusted Shops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
