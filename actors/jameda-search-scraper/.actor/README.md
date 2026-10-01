# Jameda Search Scraper

Find Jameda doctors by specialty and location, with names, ratings and practice links. Search by medical specialty and city to find public doctor profiles for a local shortlist.

## What data can you extract?

Doctor details and patient reviews follow public Jameda pages; profiles and reviews can omit optional details.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the business search result, as shown by Jameda; null when no name is published. |
| `specialty` | text | Medical specialty shown for the business search result by Jameda; null when Jameda does not provide the value. |
| `rating` | text | Rating for this business search result, on the rating scale shown by Jameda; null when no score is shown. |
| `review_count` | text | Number of reviews shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `review_count_number` | number | Number of reviews shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `address` | text | Address shown for the business search result by Jameda, in the format used by the source; null when it is omitted. |
| `profile_url` | link | Profile url for this business search result on Jameda; null when the source does not provide a URL. |
| `image_url` | image | Image url for this business search result on Jameda; null when the source does not provide a URL. |
| `request_q` | text | Search phrase passed to Jameda. This input value is copied into the output row; null when it was not supplied. |
| `request_loc` | text | Location query passed to Jameda. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Jameda; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_per_page` | number | Number of results per page passed to Jameda; A whole-number result count. This input value is copied into the output row; null when it was not supplied. |
| `total_results` | number | Number of total results shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `total_pages` | number | Number of pages shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `has_next_page` | boolean | Whether another result page is available; false is a reported value, while null means Jameda provided no flag. |

## Use cases

- Practice managers can review public provider profiles and patient feedback.
- Patients can compare doctor ratings and written feedback for a specialty.
- Healthcare researchers can summarize public review themes across practices.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `searches` and use the identifier or URL format required by Jameda.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "searches": [
    {
      "q": "Zahnarzt",
      "loc": "Berlin"
    },
    {
      "q": "Hausarzt",
      "loc": "München"
    }
  ],
  "q": "Zahnarzt",
  "loc": "Berlin",
  "page": 1,
  "per_page": 28
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `searches` | array of object | No | Recommended. Process many Jameda query/location searches in one Apify run so run startup and storage overhead are shared across doctor results. Constraints: maximum 10 items. |
| `q` | string | No | Backward-compatible single doctor name, specialty, symptom, or medical service to search on Jameda. Prefer searches for normal usage, especially when running more than one query. |
| `loc` | string | No | Optional German city or location for the legacy single query. |
| `page` | integer | No | One-based Jameda search results page. Constraints: minimum 1; maximum 500. |
| `per_page` | integer | No | Maximum doctor results to save per page. Jameda returns up to 28 results per page. Constraints: minimum 1; maximum 28. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 2. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Northstar Market Labs",
  "rating": "4.7/5",
  "review_count": "184",
  "specialty": "Family medicine",
  "review_count_number": 184,
  "address": "731 Alder Way, Seattle, WA 98103",
  "profile_url": "https://profiles.example.com/northstar-market-labs",
  "image_url": "https://images.example.com/listings/vintage-coat-01.jpg"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~jameda-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What should I enter in Jameda search?

Enter a specific topic or product in the q field. Use filters only when this source supports them.

## Related Scrappa Actors

- [Jameda Doctor Details Scraper](https://apify.com/thescrappa/jameda-doctor-details-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Google Maps Search Scraper](https://apify.com/thescrappa/google-maps-search-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
