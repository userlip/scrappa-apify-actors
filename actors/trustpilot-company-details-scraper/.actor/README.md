# Trustpilot Company Details Scraper

Fetch structured Trustpilot company profile details by domain through Scrappa. Use this Actor after Trustpilot Business Search to enrich company domains, or before Trustpilot Company Reviews to decide which profiles are worth monitoring.

## Features

- Fetch Trustpilot profile details for one company domain or a batch of domains in one Apify run
- Save one dataset item per processed company domain
- Extract company name, Trustpilot profile URL, website, TrustScore, stars, review count, claimed/verified status, location, categories, contact fields, social media, and Scrappa metadata when available
- Dataset rows optimized for Apify table views
- Full Scrappa responses saved to the compact `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `company_domain` | string | If no batch | Single company domain, for example `trustpilot.com` |
| `company_domains` | array | If no single domain | Batch of company domains. Each domain produces one dataset item |
| `locale` | string | No | Trustpilot locale. Default `en-US` |
| `fields` | string | No | Optional comma-separated Scrappa response fields if projection is enabled for this endpoint later |

`company_domain` and `company_domains` can be used together. Domains are normalized, deduplicated, and capped at 100 per run.

## Example Single Company Input

```json
{
  "company_domain": "trustpilot.com",
  "locale": "en-US"
}
```

## Example Batch Input

```json
{
  "company_domains": [
    "trustpilot.com",
    "amazon.com",
    "example.com"
  ],
  "locale": "en-US"
}
```

## Output

Each company profile is saved as one dataset item:

```json
{
  "company_name": "Trustpilot",
  "company_domain": "trustpilot.com",
  "website_url": "https://www.trustpilot.com",
  "profile_url": "https://www.trustpilot.com/review/trustpilot.com",
  "trust_score": 4.4,
  "stars": 4.5,
  "review_count": 501587,
  "is_claimed": true,
  "is_verified": false,
  "country": "United States",
  "country_code": "US",
  "city": "New York",
  "category_names": "Review Site, Software Company",
  "request_locale": "en-US",
  "scraped_at": "2026-06-06T05:03:22.364719Z"
}
```

The dataset item also includes the original Scrappa response fields such as `basic_info`, `ratings`, `location`, `categories`, `contact`, `social_media`, and `metadata`.

## Pricing

This Actor is designed for paid, usage-aligned runs. Charge per saved `company-detail-result` dataset item so users pay for delivered company profile records.

## Notes

Trustpilot details return public company profile data. For higher-volume Trustpilot discovery, profile enrichment, review monitoring, or direct API access, use Scrappa at https://scrappa.co/api/trustpilot/company-details.
