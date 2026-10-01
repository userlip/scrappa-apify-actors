# Trustpilot Company Reviews Scraper

Read public Trustpilot reviews with reviewer name, rating, title and written feedback. Choose a company domain and apply supported rating, date or reply filters to its public reviews.

## What data can you extract?

Ratings and review details follow the public Trustpilot profile; review text and owner replies vary by record.

| Field | Type | Description |
| --- | --- | --- |
| `consumer_name` | text | Reviewer display name shown for the customer or employee review by Trustpilot, in the format used by the source; null when it is omitted. |
| `rating` | number | Rating for this customer or employee review, on Trustpilot’s 1-to-5 scale; null when no score is shown. |
| `title` | text | Title of the customer or employee review, as shown by Trustpilot; null when no title is published. |
| `text` | text | Text content from Trustpilot for this customer or employee review; null when the source has no text to show. |
| `published_date` | date | Date this review was published shown by Trustpilot, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `language` | text | Language code or language name used for this text; null when Trustpilot does not provide the value. |
| `isVerified` | boolean | Whether the source marks the review as verified; false is a reported value, while null means Trustpilot provided no flag. |
| `reply` | object | Company reply with response text, date and author when present from Trustpilot; null when the source provides no details. |
| `id` | text | source ID for the customer or employee review, assigned by Trustpilot; null when the source does not expose it. |
| `review_source` | text | Review source shown for the customer or employee review by Trustpilot, in the format used by the source; null when it is omitted. |
| `company_domain` | text | Company domain shown for the customer or employee review by Trustpilot, in the format used by the source; null when it is omitted. |
| `request_page` | number | Requested result page number passed to Trustpilot; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_sort` | text | Result sort order passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `request_rating` | text | Minimum rating filter passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |

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
  "company_domain": "amazon.com",
  "page": 1,
  "per_page": 20
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `company_domain` | string | Yes | Company domain with or without protocol, for example amazon.com or https://www.amazon.com. |
| `locale` | string | No | Trustpilot locale used for the review page. Constraints: allowed values: da-DK, de-AT, de-CH, de-DE, en-AU, en-CA, en-GB, en-IE, en-NZ, en-US, es-ES, fi-FI, fr-BE, nl-BE, fr-FR, it-IT, ja-JP, nb-NO, nl-NL, pl-PL, pt-BR, pt-PT, sv-SE. |
| `page` | integer | No | First Trustpilot review page to fetch. Trustpilot limits public access to approximately the first 10 pages. Constraints: minimum 1; maximum 10. |
| `max_pages` | integer | No | Number of pages to fetch in this run. Page plus Max Pages cannot go beyond page 10. Constraints: minimum 1; maximum 10. |
| `per_page` | integer | No | Maximum reviews returned per page after Scrappa filtering. Constraints: minimum 1; maximum 100. |
| `sort` | string | No | Use recency for monitoring or relevance for representative samples. Constraints: allowed values: recency, relevance. |
| `rating` | string | No | Comma-separated ratings from 1 to 5, for example 1,2 for negative-review monitoring or 4,5 for positive reviews. |
| `verified` | boolean | No | Only return verified reviews. |
| `with_replies` | boolean | No | Only return reviews that include company replies. |
| `query` | string | No | Keyword to search in review title or text, such as refund, delivery, support, or warranty. |
| `date_posted` | string | No | Limit reviews by publication date. Constraints: allowed values: any, last_30_days, last_3_months, last_6_months, last_12_months. |
| `fields` | string | No | Optional comma-separated Scrappa response fields to include, such as reviews, relevantReviews, aiSummaryReviews, businessUnit.displayName, and pagination. Leave empty for the full response. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "consumer_name": "Avery Chen",
  "rating": 4.7,
  "title": "Helpful staff and a clear answer",
  "text": "The team answered my question clearly and kept me updated throughout the process.",
  "published_date": "2026-09-25",
  "reply": {
    "text": "Thank you for sharing this feedback. We have passed your comments to the store team.",
    "date": "2026-09-26",
    "author": "Northstar Market Labs Support"
  },
  "language": "en",
  "isVerified": true
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved review counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustpilot-company-reviews-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I choose which Trustpilot company reviews to collect?

Set `company_domain`, then use `rating`, `verified`, `with_replies`, `date_posted` or `sort` when those filters are available in Input.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
