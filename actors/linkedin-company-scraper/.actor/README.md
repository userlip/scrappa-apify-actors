# LinkedIn Company Scraper

Scrape public LinkedIn company pages without a LinkedIn login. Use this Actor to enrich company records, qualify B2B accounts, monitor competitors, and collect public company profile data from LinkedIn at a predictable price.

## What You Get

- Company name, description, logo, website, industry, type, and size
- Public follower count and employee count
- Specialties and positioning keywords from the company page
- Public office address data when LinkedIn exposes it
- Public employees, posts, similar pages, and funding data when available
- Cache controls for faster repeat runs and lower duplicate work
- One clean dataset item per company URL, plus the full response in `OUTPUT`

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | Full LinkedIn company page URL. Example: `https://www.linkedin.com/company/microsoft` |
| `use_cache` | boolean | No | Use Scrappa cached results when available. Default: `false` |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds. Only applies when `use_cache` is enabled |

### Example Input

```json
{
  "url": "https://www.linkedin.com/company/microsoft",
  "use_cache": true,
  "maximum_cache_age": 2592000
}
```

## Output

Each run pushes one company object to the default dataset and stores the same full response in the key-value store under `OUTPUT`.

### Main Fields

| Field | Description |
| --- | --- |
| `success` | Whether the scrape completed successfully |
| `name` | Company name from LinkedIn |
| `description` | Public company description/about text |
| `logo` | Company logo URL |
| `website` | Website listed on the LinkedIn page |
| `employee_count` | Public LinkedIn employee count |
| `address` | Array of office address objects when available |
| `posts` | Array of public company posts when available |
| `followers` | Public LinkedIn follower count |
| `similar_pages` | Related company pages suggested by LinkedIn |
| `specialties` | Array of specialties and topic keywords |
| `employees` | Array of public employee preview objects when available |
| `funding` | Funding object when available, otherwise `null` |
| `industry` | LinkedIn industry label |
| `size` | LinkedIn company size label |
| `type` | Company type, such as `Public Company` |
| `cached` | Whether the result came from cache |
| `cached_at` | Timestamp for the cached result when returned |

### Example Output

```json
{
  "success": true,
  "name": "Microsoft",
  "description": "Every company has a mission. What's ours? To empower every person and every organization to achieve more.",
  "logo": "https://media.licdn.com/dms/image/...",
  "website": "https://news.microsoft.com/",
  "employee_count": 229722,
  "address": [],
  "posts": [],
  "followers": 28121865,
  "similar_pages": [],
  "specialties": [
    "Business Software",
    "Developer Tools",
    "Cloud Computing",
    "AI",
    "Machine Learning"
  ],
  "employees": [],
  "funding": null,
  "industry": "Software Development",
  "size": "10,001+ employees",
  "type": "Public Company",
  "cached": true,
  "cached_at": "2026-04-28T18:34:44+00:00"
}
```

## Cache Controls

Use caching when you are refreshing the same company list or testing workflows:

- Set `use_cache` to `true` to allow cached results.
- Set `maximum_cache_age` to control freshness in seconds.
- Use `2592000` for a 30-day cache window.
- Leave `use_cache` as `false` when you need the freshest available company profile data.

## Pricing

This Actor is priced at **$0.30 per 1,000 results**. There is no LinkedIn login requirement and no extra API key needed inside Apify.

## Best For

- B2B account enrichment from LinkedIn company URLs
- Lead scoring and sales operations workflows
- Market mapping by industry, size, and specialties
- Competitive intelligence and category tracking
- CRM cleanup for company websites, descriptions, and employee counts

## Need Higher Volume?

If you need direct API access, higher-throughput enrichment, or want to run LinkedIn company scraping inside your own backend, upgrade to the Scrappa API.

Use the same data source directly:

```bash
curl "https://scrappa.co/api/linkedin/company?url=https%3A%2F%2Fwww.linkedin.com%2Fcompany%2Fmicrosoft&use_cache=1&maximum_cache_age=2592000" \
  -H "X-API-Key: YOUR_SCRAPPA_API_KEY" \
  -H "Accept: application/json"
```

Direct API access is the better fit when you need bulk company enrichment, scheduled syncs, CRM integrations, or custom retry and caching logic outside Apify.

## Support

For issues or questions, contact us through Apify.
