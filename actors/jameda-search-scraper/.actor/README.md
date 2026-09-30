# Jameda Search Scraper for Lead Generation

The Jameda Search Scraper for Lead Generation collects search results, names, and source links from Jameda. Provide a search phrase or a short list of phrases; the actor saves source fields such as `name`, `specialty`, `rating`, and `review_count` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Jameda. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name returned for this result. |
| `specialty` | text | Specialty returned for this result. |
| `rating` | text | Rating returned for this result. |
| `review_count` | text | Reviews returned for this result. |
| `review_count_number` | number | Review Count returned for this result. |
| `address` | text | Address returned for this result. |
| `profile_url` | link | Jameda Profile returned for this result. |
| `image_url` | image | Image returned for this result. |
| `request_q` | text | Request Query returned for this result. |
| `request_loc` | text | Location returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_per_page` | number | Per Page returned for this result. |
| `total_results` | number | Total Results returned for this result. |
| `total_pages` | number | Total Pages returned for this result. |
| `has_next_page` | boolean | Has Next Page returned for this result. |

## Use cases

- Collect search results, names, and source links to support lead generation.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `searches` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

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

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "name": "Example value",
  "specialty": "Example value",
  "rating": "4.7",
  "review_count": "42",
  "review_count_number": 42,
  "address": "Example location",
  "profile_url": "https://example.com/result/1"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `searches` | array of object | No | Recommended. Process many Jameda query/location searches in one Apify run so run startup and storage overhead are shared across doctor results. Constraints: maximum 10 items. |
| `q` | string | No | Backward-compatible single doctor name, specialty, symptom, or medical service to search on Jameda. Prefer searches for normal usage, especially when running more than one query. |
| `loc` | string | No | Optional German city or location for the legacy single query. |
| `page` | integer | No | One-based Jameda search results page. Constraints: minimum 1; maximum 500. |
| `per_page` | integer | No | Maximum doctor results to save per page. Jameda returns up to 28 results per page. Constraints: minimum 1; maximum 28. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 2. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Jameda. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~jameda-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Jameda Doctor Details Scraper for Lead Research](https://apify.com/thescrappa/jameda-doctor-details-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Google Maps Search Scraper for Lead Research](https://apify.com/thescrappa/google-maps-search-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
