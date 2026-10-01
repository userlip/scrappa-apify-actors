# Jameda Reviews Scraper

Read Jameda patient reviews with ratings, written feedback, posting dates and review titles. Use a doctor profile URL and choose the supported review sort and rating filters.

## What data can you extract?

Doctor details and patient reviews follow public Jameda pages; profiles and reviews can omit optional details.

| Field | Type | Description |
| --- | --- | --- |
| `review_id` | text | review ID for the customer or employee review, assigned by Jameda; null when the source does not expose it. |
| `rating` | text | Rating for this customer or employee review, on the rating scale shown by Jameda; null when no score is shown. |
| `rating_number` | number | Rating number for this customer or employee review, on the rating scale shown by Jameda; null when no score is shown. |
| `date` | date | Date shown for the customer or employee review shown by Jameda, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `date_formatted` | text | Date formatted shown for the customer or employee review by Jameda, in the format used by the source; null when it is omitted. |
| `verification_badge` | text | Verification badge shown for the customer or employee review by Jameda, in the format used by the source; null when it is omitted. |
| `review_text` | text | Written review from Jameda for this customer or employee review; null when the source has no text to show. |
| `doctor_name` | text | Doctor name shown for the customer or employee review by Jameda, in the format used by the source; null when it is omitted. |
| `doctor_specializations` | text | Doctor specialties shown for the customer or employee review by Jameda, in the format used by the source; null when it is omitted. |
| `doctor_overall_rating` | text | Doctor overall rating for this customer or employee review, on the rating scale shown by Jameda; null when no score is shown. |
| `input_doctor_url` | link | Jameda doctor profile url passed to Jameda. This input value is copied into the output row; null when it was not supplied. |
| `normalized_doctor_url` | link | Normalized doctor url for this customer or employee review on Jameda; null when the source does not provide a URL. |
| `request_page` | number | Requested result page number passed to Jameda; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_sort` | text | Result sort order passed to Jameda. This input value is copied into the output row; null when it was not supplied. |
| `request_rating` | text | Minimum rating filter passed to Jameda. This input value is copied into the output row; null when it was not supplied. |
| `request_per_page` | number | Number of results per page passed to Jameda; A whole-number result count. This input value is copied into the output row; null when it was not supplied. |
| `total_reviews` | number | Number of reviews shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `total_pages` | number | Number of pages shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `has_next_page` | boolean | Whether another result page is available; false is a reported value, while null means Jameda provided no flag. |
| `response_source` | text | Source used for the response shown for the customer or employee review by Jameda, in the format used by the source; null when it is omitted. |

## Use cases

- Practice managers can review public provider profiles and patient feedback.
- Patients can compare doctor ratings and written feedback for a specialty.
- Healthcare researchers can summarize public review themes across practices.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a company, shop, place or provider identifier and use the available sort, rating and page fields to focus the reviews.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "doctor_urls": [
    "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin"
  ],
  "page": 1,
  "rating": "4,5",
  "per_page": 20
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `doctor_urls` | array of string | No | Recommended. Process many Jameda doctor profile URLs in one Apify run. Each saved review is one dataset item. Constraints: maximum 100 items. |
| `doctor_url` | string | No | Backward-compatible single Jameda doctor profile URL or path. Prefer Doctor URLs for normal usage, especially when monitoring reviews for more than one provider. |
| `page` | integer | No | One-based reviews page to fetch for each doctor URL. Constraints: minimum 1; maximum 500. |
| `sort` | string | No | Sort order applied to available reviews. Constraints: allowed values: newest, oldest, highest, lowest. |
| `rating` | string | No | Optional rating filter. Use a single value from 1 to 5 or comma-separated values such as 4,5. |
| `per_page` | integer | No | Number of reviews to request per doctor URL. Constraints: minimum 1; maximum 100. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "review_text": "Helpful staff answered my question clearly and followed up the same day.",
  "rating": "4.7/5",
  "date": "1790294400000",
  "review_id": "review_demo_184",
  "rating_number": 4.7,
  "date_formatted": "September 25, 2026",
  "verification_badge": "Verified purchase",
  "doctor_name": "Taylor Morgan"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~jameda-reviews-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I sort or filter Jameda reviews?

Provide a doctor URL through `doctor_url` or `doctor_urls`, then select the supported `sort`, `rating` and page settings in Input.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [Trusted Shops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
- [Trusted Shops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
