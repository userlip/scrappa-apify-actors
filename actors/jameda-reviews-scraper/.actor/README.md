# Jameda Reviews Scraper

Extract public Jameda doctor reviews through Scrappa. Use this Jameda reviews scraper for reputation monitoring, medical provider benchmarking, local SEO research, patient sentiment extraction, and review intelligence workflows.

## Features

- Scrape Jameda doctor reviews from one doctor URL or a batch of doctor URLs
- Accept full `https://www.jameda.de/...` URLs, host-style URLs, or path-only inputs
- Filter reviews by page, sort order, rating, and reviews per page
- Save one Apify dataset item per review with doctor URL context
- Keep `OUTPUT` compact with request counts, saved review count, and failures

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `doctor_urls` | array | Yes, unless using `doctor_url` | Recommended input. Process multiple Jameda doctor profile URLs in one Apify run. |
| `doctor_url` | string | Yes, unless using `doctor_urls` | Legacy single Jameda doctor profile URL or path. |
| `page` | integer | No | One-based review page for each doctor URL, 1-500. Default `1`. |
| `sort` | string | No | `newest`, `oldest`, `highest`, or `lowest`. Default `newest`. |
| `rating` | string | No | Filter by rating from 1 to 5. Use comma-separated values such as `4,5`. |
| `per_page` | integer | No | Reviews to request per doctor URL, 1-100. Default `20`. |

## Example Input

```json
{
  "doctor_urls": [
    "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
    "/markus-lietzau-msc/zahnarzt/berlin"
  ],
  "page": 1,
  "sort": "newest",
  "rating": "4,5",
  "per_page": 20
}
```

## Output

Each Jameda review is saved as one dataset item:

```json
{
  "review_id": "4986285",
  "review_text": "Sehr freundlicher und kompetenter Zahnarzt!",
  "rating": "4",
  "rating_number": 4,
  "date": "2025-07-15T17:38:48+02:00",
  "date_formatted": "15. Juli 2025",
  "verification_badge": "Termin verifiziert",
  "doctor_name": "Markus Lietzau M.Sc.",
  "doctor_specializations": "Zahnarzt",
  "input_doctor_url": "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
  "normalized_doctor_url": "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin",
  "request_page": 1,
  "request_sort": "newest",
  "request_rating": "4,5",
  "request_per_page": 20,
  "total_reviews": 132,
  "total_pages": 7,
  "has_next_page": true,
  "response_source": "jameda_ajax_api"
}
```

The `OUTPUT` record includes the requested doctor URLs, reviews saved, failed requests, and status message. Review rows are written to the dataset as the primary output channel.

## Notes

Put multiple doctor URLs in `doctor_urls` when monitoring a provider group or competitor set. This shares Apify run overhead across many review results while Scrappa handles the actual Jameda scraping work. For high-volume direct API access, use Scrappa at https://scrappa.co.
