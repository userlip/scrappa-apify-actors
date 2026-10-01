# Kleinanzeigen Listing Details Scraper

Look up a Kleinanzeigen listing for its description, price, photos and seller details. Paste a Kleinanzeigen ad ID for details, or use the supported query input to search by phrase.

## What data can you extract?

Listing details follow the current Kleinanzeigen page; prices and availability can change.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the marketplace listing, assigned by Kleinanzeigen; null when the source does not expose it. |
| `title` | text | Title of the marketplace listing, as shown by Kleinanzeigen; null when no title is published. |
| `price` | text | Listed price for this marketplace listing, as a numeric amount in the listing currency; null when Kleinanzeigen provides no price. |
| `price_numeric` | number | Numeric listed price for this marketplace listing, as a numeric amount in the listing currency; null when Kleinanzeigen provides no price. |
| `description` | text | Description text from Kleinanzeigen for this marketplace listing; null when the source has no text to show. |
| `location` | text | Location shown for the marketplace listing by Kleinanzeigen, in the format used by the source; null when it is omitted. |
| `images` | array | Image URLs and image details attached to the record from Kleinanzeigen; an empty list when no entries are available. |
| `seller` | text | Seller shown for the marketplace listing by Kleinanzeigen, in the format used by the source; null when it is omitted. |
| `attributes` | text | Job attributes grouped by label and value from Kleinanzeigen; null when the source provides no details. |
| `shipping` | text | Shipping shown for the marketplace listing by Kleinanzeigen, in the format used by the source; null when it is omitted. |
| `posted_at` | text | Time the post was published shown by Kleinanzeigen, in ISO 8601 date and time; null if the source omits the date. |
| `categories` | text | Categories shown for the marketplace listing by Kleinanzeigen, in the format used by the source; null when it is omitted. |
| `request_ad_id` | text | Tiktok ad id passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted Kleinanzeigen input batch; null for a single-item lookup. |

## Use cases

- Resellers can compare listings by title, price, condition and location.
- Marketplace teams can monitor inventory for a brand or category.
- Catalog operators can collect source-linked records for product research.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `ad_ids` and use the identifier or URL format required by Kleinanzeigen.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "fahrrad",
  "ad_ids": [
    "1"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `ad_id` | string | No | A single Kleinanzeigen listing ID. It can be combined with ad_ids. |
| `ad_ids` | array of string | No | Optional batch of up to 100 listing IDs. Duplicate IDs are fetched once in first-seen order. Constraints: maximum 100 items. |
| `query` | string | No | When no ad_id or ad_ids is supplied, search this query and save the first successful listing detail (up to three candidates). Explicit IDs take priority. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "images": [
    "https://images.example.com/listings/vintage-coat-01.jpg",
    "https://images.example.com/listings/vintage-coat-02.jpg"
  ],
  "title": "Vintage wool coat in very good condition",
  "description": "Classic gray wool coat in very good condition, with a lined interior and two front pockets.",
  "price": "68.00",
  "location": "Seattle, WA",
  "id": "3176212345",
  "price_numeric": 128.5,
  "seller": "Seller for the marketplace listing on Kleinanzeigen"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~kleinanzeigen-listing-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I search Kleinanzeigen from the listing details Actor?

The main lookup uses `ad_id` or `ad_ids`. A `query` is supported for the search path described in Input; use Kleinanzeigen Search for broader listing discovery.

## Related Scrappa Actors

- [Kleinanzeigen Search Scraper](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper](https://apify.com/thescrappa/vinted-user-profile-scraper)
