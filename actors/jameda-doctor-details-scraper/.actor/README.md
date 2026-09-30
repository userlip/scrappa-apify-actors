# Jameda Doctor Details Scraper for Lead Research

The Jameda Doctor Details Scraper for Lead Research collects public record details and identifying fields from Jameda. Provide the fields listed below; the actor saves source fields such as `doctor_name`, `title`, `specialty`, and `rating` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Jameda. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `doctor_name` | text | Doctor returned for this result. |
| `title` | text | Title returned for this result. |
| `specialty` | text | Specialty returned for this result. |
| `rating` | text | Rating returned for this result. |
| `rating_number` | number | Rating Number returned for this result. |
| `review_count` | text | Reviews returned for this result. |
| `review_count_number` | number | Review Count returned for this result. |
| `clinic_name` | text | Clinic returned for this result. |
| `phone` | text | Phone returned for this result. |
| `website_url` | link | Website returned for this result. |
| `address` | text | Address returned for this result. |
| `city` | text | City returned for this result. |
| `postal_code` | text | Postal Code returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `services_count` | number | Services returned for this result. |
| `focus_areas_count` | number | Focus Areas returned for this result. |
| `conditions_count` | number | Conditions returned for this result. |
| `languages_count` | number | Languages returned for this result. |
| `doctor_url` | link | Jameda Profile returned for this result. |
| `requested_doctor_url` | link | Requested URL returned for this result. |
| `response_source` | text | Source returned for this result. |
| `scraped_at` | date | Scraped At returned for this result. |

## Use cases

- Collect public record details and identifying fields to support lead generation.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `doctorUrls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "doctorUrls": [
    "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin"
  ],
  "doctorUrl": "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "doctor_name": "Example value",
  "title": "Example result",
  "specialty": "Example value",
  "rating": "4.7",
  "rating_number": 4.7,
  "review_count": "42",
  "review_count_number": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `doctorUrls` | array of string | No | Recommended. Process many Jameda doctor profile URLs in one Apify run. Each successful doctor URL produces one dataset item. Constraints: maximum 100 items. |
| `doctorUrl` | string | No | Backward-compatible single Jameda doctor profile URL or path. Prefer Doctor URLs for normal usage, especially when enriching more than one profile. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Jameda. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~jameda-doctor-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Jameda Search Scraper for Lead Generation](https://apify.com/thescrappa/jameda-search-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Google Maps Search Scraper for Lead Research](https://apify.com/thescrappa/google-maps-search-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
