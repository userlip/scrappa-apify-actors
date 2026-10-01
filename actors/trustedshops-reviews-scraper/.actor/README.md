# TrustedShops Reviews Scraper

Read Trusted Shops reviews with shop names, ratings, written feedback and posting dates. Use a Trusted Shops shop ID or URL and page through the reviews available for that profile.

## What data can you extract?

Ratings and review details follow public Trusted Shops profiles; some shops have no review text or reply.

| Field | Type | Description |
| --- | --- | --- |
| `tsid` | text | Trusted Shops ID for the customer or employee review, assigned by Trusted Shops; null when the source does not expose it. |
| `shop_name` | text | Name of the customer or employee review, as shown by Trusted Shops; null when no name is published. |
| `rating` | number | Rating for this customer or employee review, on Trusted Shops’ 1-to-5 rating scale; null when no score is shown. |
| `review_title` | text | Review title shown for the customer or employee review by Trusted Shops, in the format used by the source; null when it is omitted. |
| `review_text` | text | Written review from Trusted Shops for this customer or employee review; null when the source has no text to show. |
| `created_at` | date | Time this record was created shown by Trusted Shops, in ISO 8601 date and time; null if the source omits the date. |
| `verified` | boolean | Whether the source marks the profile or review as verified; false is a reported value, while null means Trusted Shops provided no flag. |
| `criteria` | object | Shop review scores grouped by delivery, product quality and service from Trusted Shops; null when the source provides no details. |
| `review_id` | text | review ID for the customer or employee review, assigned by Trusted Shops; null when the source does not expose it. |
| `page` | number | Page in the Trusted Shops customer or employee review list, as a whole number; null when the source does not supply one. |
| `source_url` | link | Source page url for this customer or employee review on Trusted Shops; null when the source does not provide a URL. |
| `request_market` | text | Market passed to Trusted Shops. This input value is copied into the output row; null when it was not supplied. |
| `page_total_reviews` | number | Number of reviews shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |
| `page_total_pages` | number | Number of pages shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a company, shop, place or provider identifier and use the available sort, rating and page fields to focus the reviews.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "tsids": [
    "XFB15FFBDE1DEE7A55D292A7D48598A6A"
  ],
  "urls": [
    "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
  ],
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tsids` | array of string | No | TrustedShops shop TSIDs. Up to 50 shops per run. Constraints: maximum 50 items. |
| `urls` | array of string | No | TrustedShops shop profile URLs containing info_<TSID>. Use this for batch monitoring from profile links. Constraints: maximum 50 items. |
| `tsid` | string | No | Single TSID compatibility input. Ignored when TSIDs or Profile URLs are provided. |
| `url` | string | No | Single TrustedShops profile URL compatibility input. |
| `market` | string | No | Optional TrustedShops market parameter when supported by the Scrappa endpoint. Constraints: allowed values: DEU, GBR, AUT, CHE, NLD, ESP, ITA, FRA, BEL, POL, PRT. |
| `page` | integer | No | First TrustedShops reviews page to fetch. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of review pages to fetch per TSID. Constraints: minimum 1; maximum 25. |
| `size` | integer | No | Reviews to request per page from Scrappa. Constraints: minimum 1; maximum 100. |
| `include_raw_responses` | boolean | No | Include full Scrappa page responses in the OUTPUT key-value-store record. Leave disabled for large batch runs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "tsid": "TS-7281945",
  "shop_name": "Cedar & Pine Outfitters",
  "rating": 4.7,
  "review_title": "Helpful support and a quick response",
  "review_text": "Helpful staff answered my question clearly and followed up the same day.",
  "created_at": "2026-09-25T09:15:00Z",
  "criteria": {
    "delivery": 4.8,
    "product_quality": 4.6,
    "customer_service": 4.9
  },
  "verified": true
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved review counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustedshops-reviews-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Trusted Shops Reviews use a shop URL instead of an ID?

Yes. Submit a supported shop URL in `url` or `urls`, or use its Trusted Shops ID through `tsid` or `tsids`.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
- [TrustedShops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
