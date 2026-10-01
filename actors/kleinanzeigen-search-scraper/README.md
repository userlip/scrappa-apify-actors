# Kleinanzeigen Search Scraper

Search Kleinanzeigen listings by phrase, location, category and price range. Enter an item phrase and use the location, category and price range fields to narrow local listings.

## What data can you extract?

Listing details follow the current Kleinanzeigen page; prices and availability can change.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the marketplace listing, assigned by Kleinanzeigen; null when the source does not expose it. |
| `title` | text | Title of the marketplace listing, as shown by Kleinanzeigen; null when no title is published. |
| `price` | text | Listed price for this marketplace listing, as a numeric amount in the listing currency; null when Kleinanzeigen provides no price. |
| `price_numeric` | number | Numeric listed price for this marketplace listing, as a numeric amount in the listing currency; null when Kleinanzeigen provides no price. |
| `location` | text | Location shown for the marketplace listing by Kleinanzeigen, in the format used by the source; null when it is omitted. |
| `url` | link | Source page url for this marketplace listing on Kleinanzeigen; null when the source does not provide a URL. |
| `image_url` | image | Image url for this marketplace listing on Kleinanzeigen; null when the source does not provide a URL. |
| `description` | text | Description text from Kleinanzeigen for this marketplace listing; null when the source has no text to show. |
| `has_shipping` | boolean | Whether shipping is offered; false is a reported value, while null means Kleinanzeigen provided no flag. |
| `request_query` | text | Search phrase passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `request_location` | text | Location filter passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `request_category` | text | Category filter passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Kleinanzeigen; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_price_min` | number | Minimum price filter passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `request_price_max` | number | Maximum price filter passed to Kleinanzeigen. This input value is copied into the output row; null when it was not supplied. |
| `results_count` | number | Number of results shown by Kleinanzeigen, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Resellers can compare listings by title, price, condition and location.
- Marketplace teams can monitor inventory for a brand or category.
- Catalog operators can collect source-linked records for product research.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `searches` and use the identifier or URL format required by Kleinanzeigen.
3. Start the run and open its default dataset to inspect or download the rows.

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

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Kleinanzeigen search text, for example iphone, fahrrad, sofa, or wohnung. |
| `page` | integer | No | One-based Kleinanzeigen search results page. Constraints: minimum 1; maximum 100. |
| `location` | string | No | Optional city, district, or location filter, for example Berlin. |
| `category` | string | No | Optional category slug or keyword, for example elektronik, auto, or moebel. |
| `price_min` | integer | No | Minimum listing price in EUR. Constraints: minimum 0. |
| `price_max` | integer | No | Maximum listing price in EUR. Constraints: minimum 0. |
| `searches` | array of object | No | Optional batch of up to 25 Kleinanzeigen searches to run in a single Actor run. When provided, top-level search fields are ignored. Constraints: maximum 25 items. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Vintage wool coat in very good condition",
  "description": "Classic gray wool coat in very good condition, with a lined interior and two front pockets.",
  "price": "68.00",
  "location": "Seattle, WA",
  "url": "https://listings.example.com/record/731-alder-way",
  "id": "3176212345",
  "price_numeric": 128.5,
  "image_url": "https://images.example.com/listings/vintage-coat-01.jpg"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~kleinanzeigen-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Kleinanzeigen Search filter by area and price?

Yes. Set `location`, `category`, `price_min` or `price_max` alongside the required `query`. You can pass multiple supported searches in `searches`.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Vinted Item Details Scraper](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper](https://apify.com/thescrappa/vinted-user-profile-scraper)
