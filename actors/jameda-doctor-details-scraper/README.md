# Jameda Doctor Details Scraper

Enrich Jameda doctor profile URLs with structured doctor details using Scrappa's live `jameda-doctor-details` endpoint. The actor is a thin Apify wrapper: Scrappa performs the scraping, while Apify handles input validation, batching, dataset output, and paid result events.

## What You Get

For each successful doctor URL, the actor saves one dataset item with the original Scrappa response plus convenient top-level fields:

- Doctor name, title, specialty, description, and profile URL
- Rating and review count
- Clinic, phone, website, address, city, postal code, and coordinates
- Opening hours, services, accepted patients, focus areas, conditions, languages, and booking IDs
- Request URL, response source, and scrape timestamp

## Input

Use `doctorUrls` for batch runs. `doctorUrl` is supported for compatibility with single-profile workflows.

```json
{
  "doctorUrls": [
    "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
    "/markus-lietzau-msc/zahnarzt/berlin"
  ]
}
```

Each URL can be a full `https://www.jameda.de/...` URL or a Jameda profile path. Duplicate URLs are processed once.

## Output Example

```json
{
  "doctor_name": "Markus Lietzau M.Sc.",
  "specialty": "Zahnarzt",
  "rating": "1,0",
  "rating_number": 1,
  "review_count": "52",
  "review_count_number": 52,
  "clinic_name": "Praxis Markus Lietzau M.Sc. Zahnarzt",
  "phone": "+49 ...",
  "address": "Berlin",
  "city": "Berlin",
  "latitude": 52.5,
  "longitude": 13.4,
  "services": [],
  "accepted_patients": [],
  "focus_areas": [],
  "conditions": [],
  "languages": [],
  "booking_ids": {},
  "doctor_url": "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
  "requested_doctor_url": "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
  "response_source": "scrappa",
  "scraped_at": "2026-06-20T00:00:00Z"
}
```

The exact fields depend on what is available on the live Jameda profile.

The actor calls Scrappa once per normalized doctor URL. The endpoint returns one profile per request and does not use a pagination cursor. On pay-per-event runs, the actor charges its `doctor-profile-result` event before publishing the profile row. Failed or uncertain dataset writes are reconciled with a read and are never replayed, preventing duplicate rows. A positive `maxTotalChargeUsd` caps the combined custom-event and default dataset-item charges. An absent, null, or zero limit is unbounded. A compact request and failure summary is saved to `OUTPUT`.

## Local Development

Run the Rust actor tests from this directory with `cargo test --locked`. Build the Apify image with `docker build -f .actor/Dockerfile -t jameda-doctor-details-scraper .`. Run `python3 test/local_image_smoke.py` to build and exercise the production image against local Apify and Scrappa mocks. The actor reads input from the default key-value store and writes dataset items plus the `OUTPUT` record through the Apify API.

## Direct API Upgrade

Need higher throughput, lower latency, or direct backend integration? Use the same endpoint through Scrappa directly:

```bash
curl "https://scrappa.co/api/jameda/doctor-details?doctor_url=https%3A%2F%2Fwww.jameda.de%2Fmarkus-lietzau-msc%2Fzahnarzt%2Fberlin" \
  -H "X-API-Key: YOUR_SCRAPPA_API_KEY"
```

Scrappa keeps the scraping workload on Scrappa infrastructure; this Apify actor is only a marketplace wrapper around the API.
