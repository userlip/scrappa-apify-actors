# Vinted User Items Scraper

Browse a Vinted seller’s items with listing titles, brands, condition and prices. Submit multiple user IDs in one run to retrieve records for each target.

## What data can you extract?

Prices and listing details follow the current Vinted page; availability can change when an item sells.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the marketplace listing, assigned by Vinted; null when the source does not expose it. |
| `title` | text | Title of the marketplace listing, as shown by Vinted; null when no title is published. |
| `price_amount` | text | Listed price for this marketplace listing, as a numeric decimal amount in the listing currency; null when Vinted provides no price. |
| `price_currency` | text | Price currency code for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `total_item_price` | text | Item total for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `total_item_price_currency` | text | Total item price currency for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `shipping_price` | text | Shipping charge for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `shipping_price_currency` | text | Shipping price currency for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `brand_name` | text | Brand name shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `category_name` | text | Category name shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `size_name` | text | Size label shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `condition` | text | Item condition shown by Vinted, such as new or used; null when Vinted does not provide the value. |
| `url` | link | Source page url for this marketplace listing on Vinted; null when the source does not provide a URL. |
| `image_url` | image | Image url for this marketplace listing on Vinted; null when the source does not provide a URL. |
| `seller_id` | text | seller ID for the marketplace listing, assigned by Vinted; null when the source does not expose it. |
| `seller_login` | text | Seller login shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `favourite_count` | number | Number of favorites shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `view_count` | number | Number of video views shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `input_user_id` | text | Source user id passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Vinted; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_per_page` | number | Number of results per page passed to Vinted; A whole-number result count. This input value is copied into the output row; null when it was not supplied. |
| `request_order` | text | Result order passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `total_entries` | number | Number of entries shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `total_pages` | number | Number of pages shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Resellers can compare listings by title, price, condition and location.
- Marketplace teams can monitor inventory for a brand or category.
- Catalog operators can collect source-linked records for product research.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `user_ids` and use the identifier or URL format required by Vinted.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "page": 1,
  "per_page": 24,
  "user_ids": [
    "3132361368"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `user_id` | string | No | Single Vinted seller user ID. Use User IDs for batches. |
| `user_ids` | array of string | No | Batch of Vinted seller user IDs. Up to 100 unique IDs per run. Constraints: maximum 100 items. |
| `country` | string | No | Vinted country market. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |
| `page` | integer | No | One-based Vinted seller inventory page. Constraints: minimum 1; maximum 999. |
| `per_page` | integer | No | Number of listings to request per page. Scrappa allows up to 100. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of inventory pages to fetch for each seller, starting from Start Page. Constraints: minimum 1; maximum 20. |
| `order` | string | No | Vinted seller inventory sort order. Constraints: allowed values: newest_first, price_low_to_high, price_high_to_low, relevance. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Vintage wool coat in very good condition",
  "price_amount": "68.00",
  "view_count": 18400,
  "url": "https://listings.example.com/record/731-alder-way",
  "id": "3482719082",
  "price_currency": "EUR",
  "total_item_price": "68.00",
  "total_item_price_currency": "EUR"
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved listing or profile record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~vinted-user-items-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Vinted User Items page through a seller’s listings?

Yes. Set `user_id`, then use `page`, `per_page` and `max_pages` within their documented limits. The seller must have public items available.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Profile Scraper](https://apify.com/thescrappa/vinted-user-profile-scraper)
