# Google Maps Directions Scraper

Compare Google Maps routes by distance, estimated travel time, travel mode and route steps. Enter origin and destination locations, then compare supported travel modes and route alternatives.

## What data can you extract?

Route distance and travel time reflect the selected origin, destination and travel mode.

| Field | Type | Description |
| --- | --- | --- |
| `alternative_index` | number | Route alternative index in the Google Maps route option list, as a whole number; null when the source does not supply one. |
| `request_index` | number | Zero-based position of this request in the submitted Google Maps input batch; null for a single-item lookup. |
| `request_origin` | text | Route origin passed to Google Maps. This input value is copied into the output row; null when it was not supplied. |
| `request_destination` | text | Route destination passed to Google Maps. This input value is copied into the output row; null when it was not supplied. |
| `request_mode` | text | Route travel mode passed to Google Maps. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Maps; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Maps; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `travel_mode` | text | Travel mode used for the route, such as driving, walking, cycling or transit; null when Google Maps does not provide the value. |
| `via` | text | Road or route waypoint shown for the route option by Google Maps, in the format used by the source; null when it is omitted. |
| `distance` | number | Length of this Google Maps route, in meters; null if no distance is available. |
| `duration` | number | Estimated travel time for this Google Maps route, in seconds; null if Google Maps has no estimate. |
| `formatted_distance` | text | Route length in Google Maps display format, such as miles or kilometers; null when no route is available. |
| `formatted_duration` | text | Travel time as Google Maps displays it, such as minutes or hours; null when no estimate is available. |
| `step_coordinates` | array | Route step coordinates as decimal latitude and longitude from Google Maps; an empty list when no entries are available. |
| `trips` | array | Route legs with distance in meters, travel time in seconds and turn-by-turn steps from Google Maps; an empty list when no entries are available. |

## Use cases

- Dispatch teams can compare route length and estimated time before assigning a service visit.
- Field teams can check driving, walking, cycling or transit options for an appointment route.
- Travel planners can inspect route steps and coordinates while preparing a local itinerary.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `routes` and use the identifier or URL format required by Google Maps.
3. Start the run and open its default dataset to inspect or download the rows.

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

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `routes` | array of object | No | Preferred batch input. One request is created for each route object; duplicate requests are processed once. Maximum 10 route requests per run. Constraints: maximum 10 items. |
| `origin` | string | No | Singular compatibility input. Use routes for batches. |
| `destination` | string | No | Singular compatibility input. Use routes for batches. |
| `mode` | string | No | Travel mode: driving, walking, bicycling/cycling, or transit. Constraints: allowed values: driving, walking, bicycling, cycling, transit. |
| `hl` | string | No | Language code for route labels, such as en or de-DE. |
| `gl` | string | No | Two-letter country or region code for geo-filtering. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "travel_mode": "Driving",
  "distance": 5200,
  "duration": 1260,
  "via": "Broadway via West 81st Street",
  "formatted_distance": "3.2 mi",
  "formatted_duration": "21 min",
  "trips": [
    {
      "distance": 5200,
      "duration": 1260,
      "steps": [
        {
          "instruction": "Continue north on Broadway",
          "distance": 420,
          "duration": 96,
          "gps_coordinates": {
            "latitude": 40.7580,
            "longitude": -73.9855
          }
        }
      ]
    }
  ],
  "step_coordinates": [
    {
      "latitude": 40.7580,
      "longitude": -73.9855
    },
    {
      "latitude": 40.7812,
      "longitude": -73.9665
    }
  ]
}
```

## Pricing

**Current live price:** $0.50 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-directions-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which travel modes can Google Maps Directions compare?

The mode input accepts driving, walking, bicycling, cycling or transit. The number of route alternatives may differ by mode.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
