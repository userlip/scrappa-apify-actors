# LinkedIn Company Scraper

Scrape public LinkedIn company pages without a LinkedIn login. Use this actor to collect company profile data, websites, industries, headcount signals, follower counts, specialties, public posts, public employees, similar pages, locations, and funding details when LinkedIn exposes them.

This actor is powered by Scrappa and is priced for simple Apify runs at **$0.30 per 1,000 results**.

## Best For

- Lead generation teams enriching account lists from LinkedIn company URLs
- Sales ops teams checking company size, industry, website, and follower count
- Market researchers comparing competitors and similar LinkedIn pages
- Recruiters and talent teams mapping company pages and visible employee signals
- RevOps and data teams that need a no-login LinkedIn company enrichment step in Apify

## Tested Input

This input was tested successfully on Apify with actor `EMGCTVXuOBRERiDMf`:

```json
{
  "url": "https://www.linkedin.com/company/microsoft",
  "use_cache": true,
  "maximum_cache_age": 2592000
}
```

The actor accepts public LinkedIn company URLs in this format:

```text
https://www.linkedin.com/company/company-slug
```

Country subdomains such as `de.linkedin.com` or `uk.linkedin.com` are normalized to `www.linkedin.com`, and query strings are removed before scraping.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | Full public LinkedIn company URL, for example `https://www.linkedin.com/company/microsoft`. |
| `use_cache` | boolean | No | When enabled, asks Scrappa to return a cached result when available. Default in the Apify input form is `false`. |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds. Use `2592000` for up to 30 days. Only useful when cache is enabled. |

## Cache Behavior

By default, the actor requests fresh data. If `use_cache` is set to `true`, the actor sends `use_cache=1` to Scrappa and may return an existing Scrappa result instead of triggering a new scrape.

Use `maximum_cache_age` to control how old a cached result can be. For example, `2592000` allows cached data up to 30 days old. Cached responses include:

- `cached`: `true` or `false`
- `cached_at`: timestamp for the cached Scrappa result, when available

Cache is useful for repeated enrichment jobs, QA runs, marketplace tests, and workflows where company profile data does not need to be refreshed on every run.

## Output Fields

Each successful company scrape is pushed to the default Apify dataset and saved in the key-value store under `OUTPUT`.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether Scrappa returned a successful company result. |
| `name` | string | Company name from the LinkedIn page. |
| `description` | string | Public company description or about text when available. |
| `logo` | string | Company logo URL when available. |
| `website` | string | Website URL listed on LinkedIn. |
| `employee_count` | number | Public LinkedIn employee count signal. |
| `followers` | number | LinkedIn follower count. |
| `industry` | string | Company industry. |
| `size` | string | LinkedIn company size range, for example `10,001+ employees`. |
| `type` | string | Company type, for example `Public Company` or `Privately Held`. |
| `address` | array | Public location objects with available street, city, state, country, and postal code fields. |
| `posts` | array | Public company post summaries when available. |
| `similar_pages` | array | Similar LinkedIn company pages with `name` and `url`. |
| `specialties` | array or string | Company specialties listed on LinkedIn. Scrappa may return either a list or a comma-separated text value depending on the source page. |
| `employees` | array | Public employee preview objects with `name`, `title`, and `profile_url` when available. |
| `funding` | object or null | Funding details when LinkedIn exposes them. |
| `cached` | boolean | Whether the result came from Scrappa cache. |
| `cached_at` | string | Cache timestamp when returned by Scrappa. |
| `message` | string | Error or not-found message for unsuccessful results. |
| `status_code` | number | Status code for unsuccessful results, including `404` for not found. |

Nested arrays are returned only when LinkedIn exposes the data publicly for the requested company. Some valid company pages return empty arrays for posts, employees, addresses, or similar pages.

## Example Output

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
  "specialties": "Business Software, Developer Tools, Home & Educational Software, Tablets, Search, Advertising, Servers, Windows Operating System, Windows Applications & Platforms, Smartphones, Cloud Computing, Quantum Computing, Future of Work, Productivity, AI, Artificial Intelligence, Machine Learning, Laptops, Mixed Reality, Virtual Reality, Gaming, Developers, and IT Professional",
  "employees": [],
  "funding": null,
  "industry": "Software Development",
  "size": "10,001+ employees",
  "type": "Public Company",
  "cached": true,
  "cached_at": "2026-04-28T18:34:44+00:00"
}
```

## Direct Scrappa API For Higher Volume

Use this Apify actor when you want a managed marketplace actor, scheduled runs, datasets, webhooks, and Apify integrations.

For higher-volume enrichment, lower-latency workflows, or direct backend integration, use the Scrappa API directly:

```bash
curl "https://scrappa.co/api/linkedin/company?url=https%3A%2F%2Fwww.linkedin.com%2Fcompany%2Fmicrosoft&use_cache=1&maximum_cache_age=2592000" \
  -H "X-API-Key: YOUR_SCRAPPA_API_KEY" \
  -H "Accept: application/json"
```

Direct API is the better fit when you need to enrich many LinkedIn company URLs from your own application, control retry logic, or combine LinkedIn company data with other Scrappa endpoints. Create or manage API keys from your Scrappa dashboard.

## Notes

- No LinkedIn login is required.
- Only public LinkedIn company page data is returned.
- The actor handles not-found companies gracefully by saving a dataset item with `success: false`, `message`, and `status_code: 404`.
- Output availability depends on what LinkedIn exposes publicly for the specific company page.

## Support

For Apify run questions, open an issue on the actor page. For higher-volume direct API access, use Scrappa.
