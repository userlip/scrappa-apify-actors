# Google Patents Details Scraper

Review Google Patents records with titles, abstracts, inventors and assignees. Submit publication numbers or supported Google Patents URLs to retrieve several records together.

## What data can you extract?

Publication numbers, legal dates and patent details follow the records indexed by Google Patents.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Google Patents provided no flag. |
| `input_patent_id` | text | Patent publication id passed to Google Patents. This input value is copied into the output row; null when it was not supplied. |
| `normalized_patent_id` | text | normalized patent ID for the patent record, assigned by Google Patents; null when the source does not expose it. |
| `patent_id` | text | patent publication ID for the patent record, assigned by Google Patents; null when the source does not expose it. |
| `publication_number` | text | Publication number shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `patent_page` | link | Google patents page url for this patent record on Google Patents; null when the source does not provide a URL. |
| `title` | text | Title of the patent record, as shown by Google Patents; null when no title is published. |
| `abstract` | text | Summary of the invention from the Google Patents record; null if the publication has no abstract. |
| `inventors` | text | Inventor names shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `assignees` | text | Patent assignees shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `dates` | text | Patent dates shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `country` | text | Country shown for the patent record by Google Patents; null when Google Patents does not provide the value. |
| `language` | text | Language code or language name used for this text; null when Google Patents does not provide the value. |
| `application_number` | text | Patent application number shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `prior_art_keywords` | text | Prior-art keywords shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `links` | text | Links shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |
| `citations` | text | Number of citations shown by Google Patents, as a whole number; zero is possible, and null means no count was reported. |
| `inventor_count` | number | Number of inventors shown by Google Patents, as a whole number; zero is possible, and null means no count was reported. |
| `assignee_count` | number | Number of assignees shown by Google Patents, as a whole number; zero is possible, and null means no count was reported. |
| `citation_count` | number | Number of citations shown by Google Patents, as a whole number; zero is possible, and null means no count was reported. |
| `cached` | boolean | Whether the record came from cache; false is a reported value, while null means Google Patents provided no flag. |
| `response_time_ms` | number | Response time for this Google Patents lookup, measured in milliseconds; null when no timing value was recorded. |
| `error` | text | Diagnostic text for the Google Patents lookup; null when the request completes without an error. |
| `status_code` | number | Http status code shown for the patent record by Google Patents, in the format used by the source; null when it is omitted. |

## Use cases

- IP teams can review publication numbers, inventors and assignees while screening an invention.
- Technology researchers can compare abstracts and citations across records.
- Product teams can inspect prior-art terms before a deeper patent review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `patent_ids` and use the identifier or URL format required by Google Patents.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "patent_ids": [
    "US9789384B1"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `patent_id` | string | No | Single patent publication ID or full Google Patents ID, such as US9789384B1 or patent/US9789384B1/en. |
| `patent_ids` | array of string | No | Batch of patent publication IDs. Each item can be a short publication ID or a full Google Patents ID. |
| `url` | string | No | Single Google Patents URL, such as https://patents.google.com/patent/US9789384B1. |
| `urls` | array of string | No | Batch of Google Patents URLs to enrich in the same Apify run. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "System for measuring renewable energy output",
  "normalized_patent_id": "US2026012345A1",
  "patent_id": "US2026012345A1",
  "publication_number": "US2026012345A1",
  "patent_page": "https://patents.google.com/patent/US2026012345A1/en",
  "abstract": "A sensor system compares solar panel output with local weather readings and flags changes that may indicate maintenance needs.",
  "inventors": "Taylor Morgan; Avery Chen",
  "assignees": "Northstar Energy Systems"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each completed profile or detail lookup counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-patents-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which patent identifier should I submit to Google Patents Details?

Use a publication number such as US9789384B1, a full Google Patents ID or a supported Google Patents URL.

## Related Scrappa Actors

- [Google Patents Search Scraper](https://apify.com/thescrappa/google-patents-search-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
