# Jameda Reviews Scraper

Apify actor wrapper for Scrappa's `jameda-reviews` endpoint. It accepts one `doctor_url` or batch `doctor_urls`, fetches Jameda doctor reviews, and pushes one dataset item per review.

## Usage

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

## Development

```bash
npm install
npm test
```

The actor is intentionally batch-first to keep Apify run overhead low. Scraping stays on Scrappa infrastructure; this actor validates input, calls `https://scrappa.co/api/jameda/reviews`, and writes dataset rows.
