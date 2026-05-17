# Arbeitsagentur Jobs Scraper

Search official German Federal Employment Agency job listings with Scrappa. This actor returns clean dataset rows for job titles, occupations, employers, locations, publication dates, start dates, reference numbers, and job URLs.

## Pricing

This actor is intended to run on usage-aligned paid pricing at $0.30 per 1,000 dataset results. The actor only pushes job rows to the dataset, so billing maps directly to successful result volume.

## Input

```json
{
  "was": "Software Entwickler",
  "wo": "Berlin",
  "umkreis": 25,
  "arbeitszeit": "vz;ho",
  "veroeffentlichtseit": 7,
  "page": 1,
  "size": 25
}
```

Use `page` and `size` to paginate. The raw Scrappa response in the `OUTPUT` key-value-store record includes `maxErgebnisse`, `page`, `size`, and `facetten` when returned by Arbeitsagentur.

## Output

Each dataset item is one Arbeitsagentur job listing. The actor preserves the raw Scrappa job fields and adds table-friendly aliases:

```json
{
  "refnr": "12265-399943_JB5100405-S",
  "titel": "Software Entwickler (m/w/d)",
  "beruf": "Softwareentwickler/-in",
  "arbeitgeber": "TechGmbH",
  "arbeitsort": {
    "ort": "Berlin",
    "plz": "10115",
    "region": "Berlin",
    "land": "Deutschland",
    "koordinaten": {
      "lat": 52.531976,
      "lon": 13.386737
    },
    "entfernung": "3"
  },
  "title": "Software Entwickler (m/w/d)",
  "occupation": "Softwareentwickler/-in",
  "company_name": "TechGmbH",
  "location_formatted": "10115, Berlin, Berlin, Deutschland",
  "location_city": "Berlin",
  "postal_code": "10115",
  "region": "Berlin",
  "country": "Deutschland",
  "published_date": "2026-03-20",
  "start_date": "2026-04-01",
  "job_url": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001",
  "reference_number": "12265-399943_JB5100405-S",
  "distance_km": "3"
}
```

The complete Scrappa response, including `facetten` and pagination metadata, is saved to the key-value store under `OUTPUT`.
