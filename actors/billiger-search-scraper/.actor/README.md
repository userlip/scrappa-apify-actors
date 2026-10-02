# Billiger.de Price Comparison Search Scraper

Search Billiger.de for products and compare prices, brands, review counts, and available offers. Add several keywords to follow retail inventory across search pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `cheapest_offer_id` | Integer | Identifier of the merchant offer with the lowest listed product price. |
| `lowest_total_price_offer_ids` | Array\<Integer\> | IDs of merchant offers with the lowest total cost after shipping. |
| `userreview_count` | Integer | Number of customer reviews associated with this product. |
| `userreview_histogram` | Array\<Integer\> | Counts of customer ratings in the source rating buckets, from lowest to highest. |
| `offer_count` | Integer | Number of shop offers currently associated with the product. |
| `offer_conditions` | Array\<String\> or null | Condition labels on available offers, such as new or used. |
| `product_url` | String | Public Billiger.de URL for the product result or linked resource. |
| `brand_id` | Integer | Billiger.de identifier for the product brand. |
| `ean` | String | European Article Number for the product when supplied. |
| `deal` | String or null | Deal badge or promotion label attached to the product, when available. |
| `test_count` | Integer | Number of product tests or reviews indexed by Billiger.de. |
| `images` | Array\<Object\> or null | List of images records for this product result; each record carries `image_url`, `height`, `width`. |
| `description` | String or null | Description text for the product result as shown by Billiger.de. |
| `category` | String | Product or listing category assigned by Billiger.de. |
| `differentiators` | Array\<Integer\> or null | Feature values Billiger.de uses to distinguish this product from similar products. |
| `differentiator_data` | String or null | Structured attributes used by Billiger.de to compare this product with similar models. |
| `min_price` | Integer | Lowest listed product price before delivery costs, in the source currency. |
| `min_total_price` | Integer | Lowest offer total including the delivery cost, in the source currency. |
| `shipping_costs` | Integer | Delivery cost associated with the lowest-priced offer, in the source currency. |
| `price_per_unit` | String or null | Unit price shown for the product, in the source currency and format. |
| `total_price_per_unit` | String or null | Per-unit cost including delivery, in the product currency. |
| `price_info` | String or null | Formatted price details supplied by Billiger.de for the product. |
| `relevance` | Number | Billiger.de search relevance score for the product result. |
| `type` | String | Billiger.de result type, distinguishing product and offer records. |
| `brand` | String | Brand name assigned to the product. |
| `indexable` | Boolean or null | True when Billiger.de marks the product page as indexable. |
| `image_url_small` | String or null | Small preview image URL for the product. |
| `offers` | String or null | Offer data associated with the product in the source response. |
| `properties` | String or null | Product specification properties supplied by Billiger.de. |
| `pricehistory` | String or null | Historical price values associated with the product, when available. |
| `pricehistory_meta` | String or null | Metadata describing the product price history, when available. |
| `related_products` | String or null | Related product records returned alongside this product. |
| `product_variants` | String or null | Variant records linked to the base product. |
| `testreports` | String or null | Product test report records indexed by Billiger.de. |
| `userreviews` | String or null | Customer review records associated with the product. |
| `userreviews_trustami` | Array\<String\> | Trustami review entries associated with the product. |
| `userreviews_combined` | String or null | Combined customer review summary returned by Billiger.de. |
| `userreview_source_url_trustami` | String or null | Trustami page URL used as a source for customer reviews. |
| `name` | String | Display name of the product on Billiger.de. |
| `baseproduct_name` | String | Name of the product family before individual variants are applied. |
| `short_name` | String or null | Short product name used in compact Billiger.de labels. |
| `userreview_rating` | Number | Average customer rating reported by Billiger.de; scale follows the source. |
| `combined_grade` | Integer | Combined product grade calculated from source reviews and tests. |
| `is_deleted` | Boolean | True when Billiger.de marks the product as deleted. |
| `baseproduct_id` | Integer | Identifier of the base product family. |
| `product_variant_ids` | Array\<Integer\> | IDs of product variants linked to this base product. |
| `efficiency_labels` | Array\<Object\> or null | Energy efficiency label records, including label name and value. |
| `energy_tyre_labels` | Array\<Object\> or null | Tyre efficiency label records attached to the product. |
| `efficiency_label_image_url` | String | Image URL for the product efficiency label. |
| `test_rating` | Integer | Product test score reported by Billiger.de; scale follows the source. |
| `image_url_large` | String | Large product image URL. |
| `product_id` | Integer | Billiger.de product identifier used to request merchant offers. |
| `has_image` | Boolean | True when Billiger.de reports at least one product image. |
| `category_id` | Integer | Billiger.de category identifier assigned to the product. |
| `max_price` | Integer or null | Highest listed price among returned product offers, when reported. |
| `videos` | Array\<Object\> or null | Product video records and their source media links. |
| `category_tags` | Array\<String\> | Category labels associated with the product. |
| `cheapest_product_id` | Integer | Identifier of the product record with the lowest price in this result group. |
| `filters` | String or null | Filter and facet data that can refine the product search. |
| `product_ids` | String or null | Product identifiers represented in the result group. |
| `product_count` | Integer | Number of products represented by this result group. |
| `url` | String | Billiger.de page URL for the product result. |
| `show_brand` | Boolean | True when Billiger.de displays the brand name for this product. |
| `image` | String | Primary product image URL or source image value. |
| `products` | String or null | Product records associated with this search result. |
| `root_category_id` | Integer | Top-level Billiger.de category identifier. |
| `input_query` | String | Query submitted to retrieve this product result. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Retail analysts can compare product prices and availability across German shops.
- Ecommerce teams can monitor competing brands and offer counts for a keyword set.
- Buyers can build product shortlists from names, ratings, and direct product links.

## How to use

1. Add one product keyword to `queries` for each search.
2. Choose an optional category, sort order, or product type to refine the matches.
3. Set `maxPages` and `maxResults`, then export product rows.

```json
{
  "queries": [
    {
      "query": "iphone 15"
    }
  ],
  "page": 1,
  "page_size": 5,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Keywords for products to compare on Billiger.de. |
| `queries[].query` | string | Yes per entry | Search text. Alias: q. |
| `queries[].page` | integer | No | 1-based page number. Defaults to 1. |
| `queries[].page_size` | integer | No | Results per page. Defaults to 20, maximum 50. |
| `queries[].sort` | string | No | relevance, clickout\_relevance, price, price\_rev, rating, or disjunctive. |
| `queries[].cat` | integer | No | Billiger category ID. |
| `queries[].doctype` | string | No | auto, offer, product, or product\_offer. |
| `queries[].fuzzy` | boolean | No | Enable fuzzy matching. |
| `page` | integer | No | 1-based page number. Defaults to 1. |
| `page_size` | integer | No | Results per page. Defaults to 20, maximum 50. |
| `sort` | string | No | relevance, clickout\_relevance, price, price\_rev, rating, or disjunctive. |
| `cat` | integer | No | Billiger category ID. |
| `doctype` | string | No | auto, offer, product, or product\_offer. |
| `fuzzy` | boolean | No | Enable fuzzy matching. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "cheapest_offer_id": 8000870,
  "lowest_total_price_offer_ids": [
    584
  ],
  "userreview_count": 347,
  "userreview_histogram": [
    8
  ],
  "offer_count": 12,
  "offer_conditions": [
    "Verified source detail"
  ],
  "product_url": "https://www.billiger.de/products/northstar-aurora-1-tb",
  "brand_id": 8000826,
  "ean": "9345678901234",
  "deal": null,
  "test_count": 6,
  "images": [
    {
      "image_url": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "height": 800,
      "width": 1200,
      "purpose": "Main product image"
    }
  ],
  "description": null,
  "category": "Consumer electronics",
  "differentiators": [
    8
  ],
  "differentiator_data": null,
  "min_price": 549,
  "min_total_price": 584,
  "shipping_costs": 4,
  "price_per_unit": null,
  "total_price_per_unit": null,
  "price_info": null,
  "relevance": 12.8,
  "type": "Product",
  "brand": "Northstar",
  "indexable": true,
  "image_url_small": null,
  "offers": null,
  "properties": null,
  "pricehistory": null,
  "pricehistory_meta": null,
  "related_products": null,
  "product_variants": null,
  "testreports": null,
  "userreviews": null,
  "userreviews_trustami": [
    "Verified marketplace detail"
  ],
  "userreviews_combined": null,
  "userreview_source_url_trustami": null,
  "name": "Northstar Aurora 1 TB Smartphone",
  "baseproduct_name": "Northstar Aurora 1 TB Smartphone",
  "short_name": null,
  "userreview_rating": 4.7,
  "combined_grade": 8,
  "is_deleted": false,
  "baseproduct_id": 8000624,
  "product_variant_ids": [
    8000281
  ],
  "efficiency_labels": [
    {
      "id": 8000251,
      "name": "Northstar Aurora 1 TB Smartphone",
      "value": "USB-C interface",
      "icon": null,
      "override": null,
      "color_name": "Northstar Outdoor Supply",
      "color": "navy"
    }
  ],
  "energy_tyre_labels": [
    {
      "icon": "primary-image",
      "desc": "Main listing image and product details."
    }
  ],
  "efficiency_label_image_url": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "test_rating": 94,
  "image_url_large": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "product_id": 8000220,
  "has_image": true,
  "category_id": 8000315,
  "max_price": 629,
  "videos": [
    {
      "video_id": "https://media.northstar.invalid/video/aurora-overview.mp4",
      "key": "primary-image",
      "video_big": "https://media.northstar.invalid/video/aurora-overview.mp4",
      "video_thumbnail_big": "https://media.northstar.invalid/video/aurora-overview.mp4",
      "type": "Product",
      "purpose": "Product overview video",
      "definition": "Main listing image and product details.",
      "title": "Northstar Aurora 1 TB Smartphone",
      "description": "Compact design with dependable performance and a two-year warranty.",
      "duration": 8
    }
  ],
  "category_tags": [
    "Consumer electronics"
  ],
  "cheapest_product_id": 8000266,
  "filters": null,
  "product_ids": null,
  "product_count": 28,
  "url": "https://www.billiger.de/products/northstar-aurora-1-tb",
  "show_brand": true,
  "image": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "products": null,
  "root_category_id": 8000874,
  "input_query": "iphone 15",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~billiger-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I filter by category?

Yes. Add a Billiger.de category ID and choose a sort order or document type. Category IDs are available in Billiger.de search results.

## Related Scrappa Actors

- [Billiger.de Product Offers Scraper](https://apify.com/thescrappa/billiger-offers-scraper)
- [Geizhals Price Comparison Search Scraper](https://apify.com/thescrappa/geizhals-search-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
