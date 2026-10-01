# Google Patents Search Scraper

Search Google Patents by invention topic, inventor, assignee, date and publication status. Search by invention topic and add supported inventor, assignee or date filters.

## What data can you extract?

Publication numbers, legal dates and patent details follow the records indexed by Google Patents.

| Field | Type | Description |
| --- | --- | --- |
| `rank` | number | Result rank in the Google Patents patent search result list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the patent search result, as shown by Google Patents; null when no title is published. |
| `patent_id` | text | patent publication ID for the patent search result, assigned by Google Patents; null when the source does not expose it. |
| `publication_number` | text | Publication number shown for the patent search result by Google Patents, in the format used by the source; null when it is omitted. |
| `patent_page` | link | Google patents page url for this patent search result on Google Patents; null when the source does not provide a URL. |
| `assignee` | text | Assignee shown for the patent search result by Google Patents, in the format used by the source; null when it is omitted. |
| `inventor` | text | Inventor shown for the patent search result by Google Patents, in the format used by the source; null when it is omitted. |
| `language` | text | Language code or language name used for this text; null when Google Patents does not provide the value. |
| `priority_date` | date | Earliest priority date for the patent shown by Google Patents, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `filing_date` | date | Date the patent application was filed shown by Google Patents, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `grant_date` | date | Date the patent was granted shown by Google Patents, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `publication_date` | date | Publication date for this patent search result shown by Google Patents, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `pdf` | link | Pdf for this patent search result on Google Patents; null when the source does not provide a URL. |
| `family_countries` | text | Family countries shown for the patent search result by Google Patents, in the format used by the source; null when it is omitted. |
| `family_status_count` | number | Number of family-statuss shown by Google Patents, as a whole number; zero is possible, and null means no count was reported. |
| `request_q` | text | Search phrase passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `request_status` | text | Status filter passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `request_type` | text | Requested result type passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `request_before` | text | Pagination continuation token passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `request_after` | text | Pagination continuation token passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- IP teams can review publication numbers, inventors and assignees while screening an invention.
- Technology researchers can compare abstracts and citations across records.
- Product teams can inspect prior-art terms before a deeper patent review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `q` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "wireless charging vehicle battery",
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Google Patents search query. Supports patent keywords and Boolean-style patent search text. |
| `page` | integer | No | One-based Google Patents result page. Constraints: minimum 1; maximum 100. |
| `num` | integer | No | Number of patent results to request on the page. Constraints: minimum 1; maximum 100. |
| `sort` | string | No | Leave empty for relevance, or sort by newest or oldest results. Constraints: allowed values: new, old. |
| `country` | string | No | Comma-separated patent country codes, such as US,EP,WO. |
| `language` | string | No | Google Patents language filter, such as ENGLISH, GERMAN, or FRENCH. |
| `status` | string | No | Filter by granted patents or applications. Constraints: allowed values: GRANT, APPLICATION. |
| `type` | string | No | Filter by utility patents or design patents. Constraints: allowed values: PATENT, DESIGN. |
| `before` | string | No | Filter patents before a filing or publication date. Format: filing:YYYYMMDD or publication:YYYYMMDD. |
| `after` | string | No | Filter patents after a filing or publication date. Format: filing:YYYYMMDD or publication:YYYYMMDD. |
| `inventor` | string | No | Comma-separated inventor names for people-focused patent searches. |
| `assignee` | string | No | Comma-separated assignee or company names for IP monitoring and competitor patent tracking. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "System for measuring renewable energy output",
  "patent_id": "US2026012345A1",
  "publication_number": "US2026012345A1",
  "patent_page": "https://patents.google.com/patent/US2026012345A1/en",
  "assignee": "Northstar Energy Systems",
  "inventor": "Taylor Morgan",
  "language": "en",
  "priority_date": "2026-09-25"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-patents-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Patents Search filter by inventor or assignee?

Use the listed inventor, assignee, date and country filters with an invention query. The output links to the matching patent records.

## Related Scrappa Actors

- [Google Patents Details Scraper](https://apify.com/thescrappa/google-patents-details-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
