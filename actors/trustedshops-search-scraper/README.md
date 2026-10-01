# Trusted Shops Search Scraper

Find Trusted Shops profiles by name and compare shop ratings, review counts and links. Enter a shop name to find its public Trusted Shops profile and review summary.

## What data can you extract?

Ratings and review details follow public Trusted Shops profiles; some shops have no review text or reply.

| Field | Type | Description |
| --- | --- | --- |
| `accountName` | text | Account name shown for the business search result by Trusted Shops, in the format used by the source; null when it is omitted. |
| `shopName` | text | Name of the business search result, as shown by Trusted Shops; null when no name is published. |
| `tsID` | text | Trusted Shops ID for the business search result, assigned by Trusted Shops; null when the source does not expose it. |
| `averageRating` | number | Average shop rating for this business search result, on Trusted Shops’ 1-to-5 rating scale; null when no score is shown. |
| `reviewCount` | number | Number of reviews shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |
| `certificationState` | boolean | Whether the shop has a certification; false is a reported value, while null means Trusted Shops provided no flag. |
| `profile_url` | link | Profile url for this business search result on Trusted Shops; null when the source does not provide a URL. |
| `shop_url` | link | Shop url for this business search result on Trusted Shops; null when the source does not provide a URL. |
| `profileType` | text | Profile type shown for the business search result by Trusted Shops, in the format used by the source; null when it is omitted. |
| `category_names` | text | Category names shown for the business search result by Trusted Shops, in the format used by the source; null when it is omitted. |
| `shopDescription` | text | Shop description shown for the business search result by Trusted Shops, in the format used by the source; null when it is omitted. |
| `shopLogoUrl` | image | Shop logo url for this business search result on Trusted Shops; null when the source does not provide a URL. |
| `contractStartDate` | number | Contract start date for this business search result, as a timestamp in the source response format; null if Trusted Shops does not supply it. |
| `request_q` | text | Search phrase passed to Trusted Shops. This input value is copied into the output row; null when it was not supplied. |
| `request_market` | text | Market passed to Trusted Shops. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Trusted Shops; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `total_shop_count` | number | Number of shops shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |
| `total_page_count` | number | Number of pages shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `q` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "zalando",
  "page": 0
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Shop, brand, or domain query to search on Trusted Shops. |
| `market` | string | No | Trusted Shops target market. Constraints: allowed values: DEU, GBR, AUT, CHE, NLD, ESP, ITA, FRA, BEL, POL, PRT. |
| `page` | integer | No | Zero-based Trusted Shops search results page. Constraints: minimum 0; maximum 100. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 10. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "accountName": "Cedar & Pine Outfitters",
  "shopName": "Cedar & Pine Outfitters",
  "tsID": "TS-7281945",
  "averageRating": 4.8,
  "reviewCount": 184,
  "certificationState": true,
  "profile_url": "https://profiles.example.com/northstar-market-labs",
  "shop_url": "https://source.example.com/record/market-guide"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved source match counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustedshops-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Trusted Shops Search find a shop by name?

Enter a shop or company phrase in the search query. A matching record may contain its profile link, rating and review count when Trusted Shops provides those values.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [TrustedShops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
