# Kununu Reviews Scraper

Review kununu employer feedback with company ratings, review text and recommendation scores. Submit multiple company slugs in one run to retrieve records for each target.

## What data can you extract?

Employer ratings and review details follow public kununu pages; a review may not include every field.

| Field | Type | Description |
| --- | --- | --- |
| `company_name` | text | Employer name attached to the customer or employee review, as shown by kununu; null when the listing does not identify its employer. |
| `company_country` | text | Company country shown for the customer or employee review by kununu, in the format used by the source; null when it is omitted. |
| `company_slug` | text | Company slug shown for the customer or employee review by kununu, in the format used by the source; null when it is omitted. |
| `rating` | number | Rating for this customer or employee review, on kununu’s employer rating scale; null when no score is shown. |
| `rounded_rating` | number | Rounded rating for this customer or employee review, on kununu’s employer rating scale; null when no score is shown. |
| `title` | text | Title of the customer or employee review, as shown by kununu; null when no title is published. |
| `text` | text | Text content from kununu for this customer or employee review; null when the source has no text to show. |
| `date` | date | Date shown for the customer or employee review shown by kununu, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `review_type` | text | Review type shown for the customer or employee review by kununu; null when kununu does not provide the value. |
| `reviewer_position` | text | Reviewer job title shown for the customer or employee review by kununu, in the format used by the source; null when it is omitted. |
| `reviewer_department` | text | Reviewer department shown for the customer or employee review by kununu, in the format used by the source; null when it is omitted. |
| `reviewer_employment_status` | text | Reviewer employment status shown for the customer or employee review by kununu, in the format used by the source; null when it is omitted. |
| `reviewer_recommended` | boolean | Whether the reviewer recommends the employer; false is a reported value, while null means kununu provided no flag. |
| `review_id` | text | review ID for the customer or employee review, assigned by kununu; null when the source does not expose it. |
| `page` | number | Page in the kununu customer or employee review list, as a whole number; null when the source does not supply one. |
| `source_url` | link | Source page url for this customer or employee review on kununu; null when the source does not provide a URL. |

## Use cases

- Employer-brand teams can review public employee feedback and ratings for a company.
- Job seekers can compare review themes and recommendations across employers.
- Workforce researchers can track public workplace feedback over time.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a company, shop, place or provider identifier and use the available sort, rating and page fields to focus the reviews.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "targets": [
    "de/bmwgroup"
  ],
  "company_slug": "bmwgroup",
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `targets` | array of string | No | Kununu company slugs, country/slug pairs, or full Kununu company URLs. Up to 25 companies per run. Constraints: maximum 25 items. |
| `company_slug` | string | No | Single Kununu company slug for compatibility. Ignored when Companies is provided. |
| `country` | string | No | Kununu country for bare slugs. Kununu supports Germany, Austria, and Switzerland. Constraints: allowed values: de, at, ch. |
| `page` | integer | No | First Kununu review page to fetch. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of pages to fetch per company target. Constraints: minimum 1; maximum 25. |
| `review_type` | string | No | Employee reviews or candidate interview reviews. Constraints: allowed values: employees, candidates. |
| `sort` | string | No | Optional Kununu sort order. Leave empty for Kununu relevance order. Constraints: allowed values: newest, oldest, best, worst. |
| `score_filters` | array of string | No | Filter by Kununu score buckets. |
| `recommended_filters` | array of string | No | Filter by recommendation status. |
| `jobstatus_filters` | array of string | No | Filter employee reviews by current or former employees. |
| `position_filters` | array of string | No | Filter by reviewer position. |
| `department_filters` | array of string | No | Filter by department. |
| `response_filters` | array of string | No | Filter by whether the company responded. |
| `date_filters` | array of string | No | Filter by review age. |
| `fetch_factor_scores` | boolean | No | Include detailed factor ratings when Kununu provides them. |
| `include_raw_review` | boolean | No | Include the full Scrappa review object in each dataset item. Leave disabled for smaller, cheaper dataset output. |
| `include_raw_responses` | boolean | No | Include full per-page Scrappa responses in the OUTPUT key-value-store record. Leave disabled for large batch runs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Helpful staff and a clear answer",
  "company_name": "Northstar Market Labs",
  "text": "The team answered my question clearly and kept me updated throughout the process.",
  "rating": 4.7,
  "date": "2026-09-25",
  "company_country": "Germany",
  "company_slug": "northstar-market-labs",
  "rounded_rating": 4.5
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~kununu-reviews-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which kununu value should I submit?

Provide the targets value listed in the Input table, using the format shown there.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
- [TrustedShops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
