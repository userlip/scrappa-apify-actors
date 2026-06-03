# Jameda Search Scraper

Search Jameda doctor profiles by specialty, doctor name, symptom, medical service, and optional German location through Scrappa. Use it for doctor discovery, local healthcare lead research, review workflows, and profile enrichment pipelines.

## Features

- Search Jameda by specialty, service, symptom, or doctor name
- Optional German city/location targeting
- Batch multiple query/location searches in one Apify run
- Fetch one or more one-based search result pages per run
- Extract doctor name, specialty, profile URL, rating, review count, address, and image URL
- Dataset rows optimized for Apify table views
- Full Scrappa page responses saved to `OUTPUT` for single-search runs; batch runs write compact summaries

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `searches` | array | Yes, unless using legacy `q` | Recommended input. Process multiple Jameda query/location searches in one Apify run. Each item has `q` and optional `loc`. |
| `q` | string | Yes, unless using `searches` | Legacy single-search query, for example `Zahnarzt`. Use `searches` for normal usage, especially when running more than one query. |
| `loc` | string | No | German city or location for the legacy single query, for example `Berlin`. |
| `page` | integer | No | First one-based result page for every search, 1-500. Default `1` |
| `per_page` | integer | No | Results to save per page, 1-28. Default `28` |
| `max_pages` | integer | No | Number of pages to fetch per search, 1-2. Default `1` |

## Example Input

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

The `OUTPUT` record includes per-search summaries, pages fetched, doctor count, and reported Jameda totals. Single-search runs also include raw Scrappa responses for debugging and auditability; batch runs keep `OUTPUT` compact because doctor records are already saved to the dataset.

## Notes

Jameda search returns public doctor profile summaries. Put multiple query/location pairs in `searches` when you have a list so Apify run overhead is shared across many results. For doctor details, reviews, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.
