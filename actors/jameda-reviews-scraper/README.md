# Jameda Reviews Scraper for Lead Research

The Jameda Reviews Scraper for Lead Research collects reviews, ratings, and comment details from Jameda. Provide the fields listed below; the actor saves source fields such as `review_id`, `rating`, `rating_number`, and `date` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Jameda. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `review_id` | text | Review ID returned for this result. |
| `rating` | text | Rating returned for this result. |
| `rating_number` | number | Rating Number returned for this result. |
| `date` | date | Date returned for this result. |
| `date_formatted` | text | Date Text returned for this result. |
| `verification_badge` | text | Verified returned for this result. |
| `review_text` | text | Review returned for this result. |
| `doctor_name` | text | Doctor returned for this result. |
| `doctor_specializations` | text | Specialization returned for this result. |
| `doctor_overall_rating` | text | Doctor Rating returned for this result. |
| `input_doctor_url` | link | Input URL returned for this result. |
| `normalized_doctor_url` | link | Doctor URL returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_sort` | text | Sort returned for this result. |
| `request_rating` | text | Rating Filter returned for this result. |
| `request_per_page` | number | Per Page returned for this result. |
| `total_reviews` | number | Total Reviews returned for this result. |
| `total_pages` | number | Total Pages returned for this result. |
| `has_next_page` | boolean | Has Next returned for this result. |
| `response_source` | text | Source returned for this result. |

## Use cases

- Collect reviews, ratings, and comment details to support lead generation.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `doctor_urls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

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

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "review_id": "Example public text.",
  "rating": "4.7",
  "rating_number": 4.7,
  "date": "2026-09-30T10:00:00Z",
  "date_formatted": "2026-09-30T10:00:00Z",
  "verification_badge": "Example value",
  "review_text": "Example public text."
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `doctor_urls` | array of string | No | Recommended. Process many Jameda doctor profile URLs in one Apify run. Each saved review is one dataset item. Constraints: maximum 100 items. |
| `doctor_url` | string | No | Backward-compatible single Jameda doctor profile URL or path. Prefer Doctor URLs for normal usage, especially when monitoring reviews for more than one provider. |
| `page` | integer | No | One-based reviews page to fetch for each doctor URL. Constraints: minimum 1; maximum 500. |
| `sort` | string | No | Sort order applied to available reviews. Constraints: allowed values: newest, oldest, highest, lowest. |
| `rating` | string | No | Optional rating filter. Use a single value from 1 to 5 or comma-separated values such as 4,5. |
| `per_page` | integer | No | Number of reviews to request per doctor URL. Constraints: minimum 1; maximum 100. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Jameda. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~jameda-reviews-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)
- [TrustedShops Shop Profile Scraper for Marketing](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
