# Trustpilot Business Search Scraper

Find Trustpilot businesses by name and compare star ratings, TrustScores and review counts. Search a company name and apply supported market, rating or review-count filters to the business results.

## What data can you extract?

Ratings and review details follow the public Trustpilot profile; review text and owner replies vary by record.

| Field | Type | Description |
| --- | --- | --- |
| `business_name` | text | Name of the business search result, as shown by Trustpilot; null when no name is published. |
| `identifying_name` | text | identifying name for the business search result, assigned by Trustpilot; null when the source does not expose it. |
| `trust_score` | number | Trustpilot score for this business search result, on Trustpilot’s 1-to-5 scale; null when no score is shown. |
| `stars` | number | Star rating for this business search result, on Trustpilot’s 1-to-5 scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Trustpilot, as a whole number; zero is possible, and null means no count was reported. |
| `is_claimed` | boolean | Whether the business has claimed its profile; false is a reported value, while null means Trustpilot provided no flag. |
| `is_verified` | boolean | Whether the source marks the profile as verified; false is a reported value, while null means Trustpilot provided no flag. |
| `website_url` | link | Website url for this business search result on Trustpilot; null when the source does not provide a URL. |
| `email` | text | Public email shown by Trustpilot; null when the profile or listing does not publish contact details. |
| `phone` | text | Public phone shown by Trustpilot; null when the profile or listing does not publish contact details. |
| `logo_url` | image | Logo url for this business search result on Trustpilot; null when the source does not provide a URL. |
| `profile_url` | link | Profile url for this business search result on Trustpilot; null when the source does not provide a URL. |
| `country` | text | Country shown for the business search result by Trustpilot; null when Trustpilot does not provide the value. |
| `country_code` | text | Country code shown for the business search result by Trustpilot; null when Trustpilot does not provide the value. |
| `city` | text | City shown for the business search result by Trustpilot; null when Trustpilot does not provide the value. |
| `category_names` | text | Category names shown for the business search result by Trustpilot, in the format used by the source; null when it is omitted. |
| `category_slugs` | text | Category slugs shown for the business search result by Trustpilot, in the format used by the source; null when it is omitted. |
| `request_search_type` | text | Search category passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `request_query` | text | Search phrase passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `request_category` | text | Category filter passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Trustpilot; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `total_results` | number | Number of total results shown by Trustpilot, as a whole number; zero is possible, and null means no count was reported. |
| `total_pages` | number | Number of pages shown by Trustpilot, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `query` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "amazon",
  "category": "electronics_technology",
  "country": "US",
  "page": 1,
  "per_page": 20,
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `search_type` | string | No | Use company search for name/domain discovery, or category search for Trustpilot category browsing. Constraints: allowed values: company_search, category. |
| `query` | string | No | Company, brand, or domain query for Trustpilot company search. Required when Search Type is Company search. |
| `category` | string | No | Trustpilot category slug, for example electronics_technology or restaurants_bars. Required when Search Type is Category businesses. |
| `locale` | string | No | Trustpilot locale for company search. Constraints: allowed values: da-DK, de-AT, de-CH, de-DE, en-AU, en-CA, en-GB, en-IE, en-NZ, en-US, es-ES, fi-FI, fr-BE, nl-BE, fr-FR, it-IT, ja-JP, nb-NO, nl-NL, pl-PL, pt-BR, pt-PT, sv-SE. |
| `country` | string | No | Optional ISO-2 country filter, for example US, GB, DE, FR, or NL. |
| `page` | integer | No | First one-based Trustpilot result page to fetch. Constraints: minimum 1; maximum 999. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 10. |
| `per_page` | integer | No | Company-search results per page. Constraints: minimum 1; maximum 50. |
| `min_rating` | number | No | Optional minimum TrustScore for company search. Constraints: minimum 0; maximum 5. |
| `min_review_count` | integer | No | Optional minimum review count for company search. Constraints: minimum 0. |
| `sort` | string | No | Optional category search sort order. Constraints: allowed values: reviews_count, latest_review. |
| `claimed` | boolean | No | Filter category search results to claimed businesses. |
| `limit` | integer | No | Maximum category businesses per page. Constraints: minimum 1; maximum 50. |
| `trustscore` | number | No | Optional minimum TrustScore for category search. Constraints: minimum 0; maximum 5. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "business_name": "Juniper Street Coffee",
  "review_count": 184,
  "identifying_name": "northstar-market-labs",
  "trust_score": 4.6,
  "stars": 4.5,
  "is_claimed": true,
  "is_verified": true,
  "website_url": "https://northstar.example"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustpilot-business-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I filter Trustpilot business results by rating?

Use the `min_rating`, `min_review_count`, `sort` and other listed filters with a business `query`. Only supported filter values are applied.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
