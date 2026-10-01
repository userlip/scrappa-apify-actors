# Vinted Item Details Scraper

Review a Vinted item with its title, description, price, size and condition. Submit multiple listing IDs in one run to retrieve records for each target.

## What data can you extract?

Prices and listing details follow the current Vinted page; availability can change when an item sells.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the marketplace listing, assigned by Vinted; null when the source does not expose it. |
| `title` | text | Title of the marketplace listing, as shown by Vinted; null when no title is published. |
| `description` | text | Description text from Vinted for this marketplace listing; null when the source has no text to show. |
| `price_amount` | text | Listed price for this marketplace listing, as a numeric decimal amount in the listing currency; null when Vinted provides no price. |
| `price_currency` | text | Price currency code for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `total_item_price` | text | Item total for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `shipping_price` | text | Shipping charge for this marketplace listing, formatted as Vinted displays it, including the currency when shown; null when unavailable. |
| `brand_name` | text | Brand name shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `category_name` | text | Category name shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `size_name` | text | Size label shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `condition` | text | Item condition shown by Vinted, such as new or used; null when Vinted does not provide the value. |
| `availability` | text | Status reported for the marketplace listing by Vinted; null when Vinted does not provide the value. |
| `url` | link | Source page url for this marketplace listing on Vinted; null when the source does not provide a URL. |
| `image_url` | image | Image url for this marketplace listing on Vinted; null when the source does not provide a URL. |
| `seller_login` | text | Seller login shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `seller_feedback_reputation` | number | Seller feedback reputation shown for the marketplace listing by Vinted, in the format used by the source; null when it is omitted. |
| `favourite_count` | number | Number of favorites shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `view_count` | number | Number of video views shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `request_item_id` | text | Marketplace item id passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted Vinted input batch; null for a single-item lookup. |
| `request_success` | boolean | Lookup success flag passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `error_message` | text | Diagnostic text for the Vinted lookup; null when the request completes without an error. |

## Use cases

- Resellers can compare listings by title, price, condition and location.
- Marketplace teams can monitor inventory for a brand or category.
- Catalog operators can collect source-linked records for product research.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `item_ids` and use the identifier or URL format required by Vinted.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "item_ids": [
    "10198495179"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `item_id` | string | No | Single Vinted item ID. Use item_ids for batches discovered from Vinted Search. |
| `item_ids` | array of string | No | Batch of Vinted item IDs. One dataset row is written per processed ID. Maximum 50 IDs per run. Constraints: maximum 50 items. |
| `country` | string | No | Vinted country market. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Vintage wool coat in very good condition",
  "description": "Classic gray wool coat in very good condition, with a lined interior and two front pockets.",
  "price_amount": "68.00",
  "view_count": 18400,
  "url": "https://listings.example.com/record/731-alder-way",
  "id": "3482719082",
  "price_currency": "EUR",
  "total_item_price": "68.00"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~vinted-item-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I find a Vinted item ID?

Copy the listing ID from a public Vinted item URL or use Vinted Search to locate the listing. Submit one ID or a batch in `item_ids`.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Search Scraper](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper](https://apify.com/thescrappa/vinted-user-profile-scraper)
