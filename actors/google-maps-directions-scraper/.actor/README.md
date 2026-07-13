# Google Maps Directions Scraper

Compare Google Maps route alternatives through Scrappa. Use driving, walking, cycling, or transit directions for travel planning, delivery estimates, commute comparisons, and logistics workflows.

This is a thin, paid Apify wrapper around Scrappa's Maps Directions endpoint (`/api/maps/directions`). Scraping runs on Scrappa infrastructure; one Apify run can process a batch of route requests.

## Features

- Process up to 10 unique origin/destination requests in one run
- Compare driving, walking, bicycling/cycling, and transit routes
- Receive one dataset row per returned route alternative
- Preserve distance, duration, formatted values, via labels, trips, step coordinates, travel mode, and source request metadata when available
- Continue after an individual route failure and report requested, succeeded, failed, saved, and charged counts in the run log
- Charge **$0.0005 per successfully stored route alternative** through the `route-result` event
- Keep failed requests and empty or malformed responses uncharged

## Input

Use the preferred `routes` array for batches:

```json
{
  "routes": [
    {
      "origin": "Berlin Hauptbahnhof",
      "destination": "Brandenburg Gate",
      "mode": "walking",
      "hl": "en",
      "gl": "de"
    },
    {
      "origin": "Alexanderplatz, Berlin",
      "destination": "Berlin Airport",
      "mode": "transit"
    }
  ]
}
```

`origin` and `destination` are also accepted as singular compatibility fields, together with singular `mode`, `hl`, and `gl`. Locations are trimmed, equivalent requests are deduplicated in first-seen order, and a run accepts at most 10 unique routes. `mode` defaults to `driving`, `hl` defaults to `en`, and `cycling` is sent to Scrappa as `bicycling`.

## Output

Each returned alternative is one dataset item. The row retains the Scrappa alternative payload and includes stable metadata:

```json
{
  "alternative_index": 0,
  "request_index": 0,
  "request_origin": "Berlin Hauptbahnhof",
  "request_destination": "Brandenburg Gate",
  "request_mode": "walking",
  "request_hl": "en",
  "request_gl": "de",
  "travel_mode": "Walking",
  "via": "B2/B5",
  "distance": 1821,
  "duration": 1501,
  "formatted_distance": "1.8 km",
  "formatted_duration": "25 min",
  "step_coordinates": [{ "latitude": 52.52104335, "longitude": 13.37325815 }],
  "trips": []
}
```

Optional response fields are preserved when Scrappa returns them; missing mode-specific details are not fabricated. Failed routes appear in the run summary and do not create charged route rows. Dataset output is the primary result channel and the actor does not write per-item key-value-store records.

## Direct API

For higher-volume routing, recurring logistics workflows, and direct API access, upgrade to Scrappa at https://scrappa.co. The underlying endpoint is `/api/maps/directions`.
