# Jameda Doctor Details Scraper

Enrich one or more Jameda doctor profile URLs with complete structured profile data. This actor calls Scrappa's live `jameda-doctor-details` endpoint and saves one Apify dataset item per successful doctor URL.

## Input

```json
{
  "doctorUrls": [
    "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
    "/markus-lietzau-msc/zahnarzt/berlin"
  ]
}
```

Prefer `doctorUrls` for batch enrichment. `doctorUrl` is available for single-profile compatibility.

## Output

Results include doctor name, specialty, description, rating, clinic, contact, address, coordinates, opening hours, services, accepted patients, focus areas, conditions, languages, booking IDs, profile URL, and scrape metadata.

For high-volume direct API access, use Scrappa at `https://scrappa.co/api/jameda/doctor-details`.
