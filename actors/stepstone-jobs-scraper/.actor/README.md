# Stepstone Jobs Scraper

Search Stepstone job listings across Germany, Austria, Netherlands, and Belgium with Scrappa. This actor returns clean dataset rows for job titles, companies, locations, salaries, posting dates, skills, labels, job URLs, and pagination metadata.

## Pricing

This actor is intended to run on usage-aligned paid pricing at $0.30 per 1,000 dataset results. The actor only pushes job rows to the dataset, so billing maps directly to successful result volume.

## Input

```json
{
  "query": "software engineer",
  "location": "Berlin",
  "country": "de",
  "sort": "date",
  "limit": 25,
  "page": 1
}
```

Supported countries are `de`, `at`, `nl`, and `be`. Use `page` with `data.pagination.next_page` from the raw `OUTPUT` key-value-store record to continue pagination.

## Output

Each dataset item is one Stepstone job listing. The actor preserves the raw Scrappa job fields and adds table-friendly aliases:

```json
{
  "id": "12345678",
  "title": "Senior Software Engineer (m/w/d)",
  "url": "https://www.stepstone.de/jobs----12345678-inline.html",
  "company": {
    "id": 9876,
    "name": "TechGmbH",
    "logo_url": "https://cdn.stepstone.de/logo/9876.png",
    "url": "https://www.stepstone.de/cmp/techgmbh"
  },
  "company_name": "TechGmbH",
  "company_url": "https://www.stepstone.de/cmp/techgmbh",
  "location": {
    "formatted": "Berlin",
    "city": "Berlin",
    "region": null,
    "country": null
  },
  "location_formatted": "Berlin",
  "location_city": "Berlin",
  "location_region": null,
  "location_country": null,
  "salary": null,
  "date_posted": "2026-03-19T08:00:00+01:00",
  "description": "We are looking for a Senior Software Engineer...",
  "skills": ["Python", "Docker", "Kubernetes"],
  "labels": ["Home office possible"],
  "work_from_home": true,
  "is_highlighted": false,
  "is_sponsored": false
}
```

The complete Scrappa response, including `pagination` and `metadata`, is saved to the key-value store under `OUTPUT`.
