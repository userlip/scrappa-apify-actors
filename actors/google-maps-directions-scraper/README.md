# Google Maps Directions Scraper for Travel Planning

The Google Maps Directions Scraper for Travel Planning collects route distances, durations, and directions from Google Maps. Provide one or more route pairs; the actor saves source fields such as `alternative_index`, `request_index`, `request_origin`, and `request_destination` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Maps. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `alternative_index` | number | Alternative returned for this result. |
| `request_index` | number | Request returned for this result. |
| `request_origin` | text | Origin returned for this result. |
| `request_destination` | text | Destination returned for this result. |
| `request_mode` | text | Requested mode returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_gl` | text | Region returned for this result. |
| `travel_mode` | text | Travel mode returned for this result. |
| `via` | text | Via returned for this result. |
| `distance` | number | Distance (m) returned for this result. |
| `duration` | number | Duration (s) returned for this result. |
| `formatted_distance` | text | Distance returned for this result. |
| `formatted_duration` | text | Duration returned for this result. |
| `step_coordinates` | array | Step coordinates returned for this result. |
| `trips` | array | Trips returned for this result. |

## Use cases

- Collect route distances, durations, and directions for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `routes` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "origin": "Times Square, New York, NY",
  "destination": "Central Park, New York, NY",
  "routes": [
    {
      "origin": "Times Square, New York, NY",
      "destination": "Central Park, New York, NY",
      "mode": "driving",
      "hl": "en"
    }
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "alternative_index": 42,
  "request_index": 42,
  "request_origin": "Example value",
  "request_destination": "Example value",
  "request_mode": "Example value",
  "request_hl": "Example value",
  "request_gl": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `routes` | array of object | No | Preferred batch input. One request is created for each route object; duplicate requests are processed once. Maximum 10 route requests per run. Constraints: maximum 10 items. |
| `origin` | string | No | Singular compatibility input. Use routes for batches. |
| `destination` | string | No | Singular compatibility input. Use routes for batches. |
| `mode` | string | No | Travel mode: driving, walking, bicycling/cycling, or transit. Constraints: allowed values: driving, walking, bicycling, cycling, transit. |
| `hl` | string | No | Language code for route labels, such as en or de-DE. |
| `gl` | string | No | Two-letter country or region code for geo-filtering. |

## Pricing

**Current live price:** $0.50 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Maps. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-maps-directions-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper for Sales](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper for Local Search](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper for Sales](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Photos Scraper for Place Research](https://apify.com/thescrappa/google-maps-photos-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
