# Kununu Jobs Scraper

Search Kununu job listings across Germany, Austria, and Switzerland with Scrappa. This actor is built for DACH hiring-market research, employer-brand monitoring, and sourcing workflows where company quality signals matter alongside job search filters.

Use it to find Kununu jobs by query, city, country, company score, workplace model, benefits, employment type, career level, industry, discipline, and Top Company badge status.

## Pricing

This actor is intended to run on usage-aligned paid pricing at $0.30 per 1,000 saved job results. The actor writes one dataset item per Kununu job listing, so billing maps directly to successful result volume.

## Input Examples

Berlin software engineering jobs:

```json
{
  "query": "Software Engineer",
  "location": "Berlin",
  "country": "de",
  "page": 1
}
```

Remote and hybrid DACH jobs with strong company scores:

```json
{
  "query": "Product Manager",
  "location": "Munich",
  "country": "de",
  "radius": 100,
  "workplace": ["FULL_REMOTE", "PARTLY_REMOTE"],
  "kununu_score": ["4-5", "3-4"],
  "sort": "kununuScore"
}
```

Top Company employers with selected benefits:

```json
{
  "query": "Data Analyst",
  "location": "Vienna",
  "country": "at",
  "is_top_company": true,
  "benefits": ["flexWorkingHours", "pensionPlan", "healthProgram"]
}
```

Supported countries are `de`, `at`, and `ch`. Kununu returns about 30 results per page. Use `page` to fetch additional result pages.

## Output

Each dataset item is one Kununu job listing. The actor preserves Scrappa job fields and adds table-friendly aliases for common marketplace columns:

```json
{
  "id": "123456",
  "title": "Senior Software Engineer (m/w/d)",
  "job_url": "https://www.kununu.com/de/example/jobs/senior-software-engineer-123456",
  "company": {
    "name": "Example GmbH",
    "slug": "example",
    "url": "https://www.kununu.com/de/example",
    "kununu_score": 4.3,
    "is_top_company": true
  },
  "company_name": "Example GmbH",
  "company_slug": "example",
  "company_url": "https://www.kununu.com/de/example",
  "company_score": 4.3,
  "company_is_top_company": true,
  "location": {
    "formatted": "Berlin",
    "city": "Berlin",
    "country": "DE"
  },
  "location_formatted": "Berlin",
  "location_city": "Berlin",
  "location_country": "DE",
  "workplace": "PARTLY_REMOTE",
  "employment_type": "FULL_TIME",
  "career_level": "3",
  "benefits": ["flexWorkingHours", "pensionPlan"],
  "date_posted": "2026-07-01"
}
```

The complete Scrappa response, including pagination and metadata when provided, is saved to the key-value store under `OUTPUT`.
