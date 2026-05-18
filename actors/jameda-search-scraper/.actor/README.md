# Jameda Search Scraper

Search Jameda doctor profiles by specialty, doctor name, symptom, medical service, and optional German location through Scrappa. Use it for doctor discovery, local healthcare lead research, review workflows, and profile enrichment pipelines.

## Features

- Search Jameda by specialty, service, symptom, or doctor name
- Optional German city/location targeting
- Fetch one or more one-based search result pages per run
- Extract doctor name, specialty, profile URL, rating, review count, address, and image URL
- Dataset rows optimized for Apify table views
- Full Scrappa page responses saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | Yes | Doctor name, specialty, symptom, or medical service, for example `Zahnarzt` |
| `loc` | string | No | German city or location, for example `Berlin` |
| `page` | integer | No | First one-based result page, 1-500. Default `1` |
| `per_page` | integer | No | Results to save per page, 1-28. Default `28` |
| `max_pages` | integer | No | Number of pages to fetch, 1-10. Default `1` |

## Example Input

```json
{
  "q": "Zahnarzt",
  "loc": "Berlin",
  "page": 1,
  "per_page": 28,
  "max_pages": 2
}
```

## Output

Each doctor profile is saved as one dataset item:

```json
{
  "name": "Dr. med. Beispiel",
  "specialty": "Zahnarzt",
  "rating": "1,2",
  "review_count": "35 Bewertungen",
  "review_count_number": 35,
  "address": "Beispielstr. 1, 10115 Berlin",
  "profile_url": "https://www.jameda.de/beispiel/zahnarzt/berlin",
  "image_url": "https://www.jameda.de/example.jpg",
  "request_q": "Zahnarzt",
  "request_loc": "Berlin",
  "request_page": 1,
  "request_per_page": 28,
  "total_results": 120,
  "total_pages": 5
}
```

The `OUTPUT` record includes the request summary, pages fetched, doctor count, reported Jameda totals, and raw Scrappa responses for the fetched pages.

## Notes

Jameda search returns public doctor profile summaries. For doctor details, reviews, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.
