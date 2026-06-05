# Google Trends Autocomplete Scraper

Scrappa-powered Apify Actor for Google Trends autocomplete suggestions.

This actor expands one seed keyword into Google Trends suggestions. It is separate from the Google Trends Interest Scraper and Google Trends Related Queries Scraper, and is built for users who need fast keyword ideation before deeper trend analysis.

Recommended paid pricing: **$0.20 per 1,000 saved suggestions** using the `suggestion-result` pay-per-event charge. Verify active or earliest scheduled paid pricing in Apify before public launch.

## What You Get

- Google Trends autocomplete suggestions for a seed keyword
- One Apify dataset row per suggestion
- Request context on every row: source keyword, location, and language
- A single `OUTPUT` summary with suggestion count and raw Scrappa response

Autocomplete suggestions vary by geography and language. For reliable first tests, use broad keywords such as `tesla`, `coffee`, `bitcoin`, or `fitness`.

## Input Example

```json
{
  "query": "tesla",
  "geo": "US",
  "hl": "en"
}
```

The actor also accepts `q` as an alias for `query` when reusing direct Scrappa API inputs.

## Dataset Output

```json
{
  "position": 1,
  "suggestion": "tesla stock",
  "type": null,
  "source_keyword": "tesla",
  "request_geo": "US",
  "request_hl": "en",
  "response_time_ms": 412,
  "search_parameters": {
    "q": "tesla",
    "geo": "US",
    "hl": "en"
  }
}
```

The actor preserves raw suggestion fields returned by Scrappa, so additional fields may appear in dataset rows when Google Trends provides richer suggestion objects.

## Use Cases

SEO keyword expansion:

```json
{
  "query": "project management software",
  "geo": "US",
  "hl": "en"
}
```

Content ideation:

```json
{
  "query": "air fryer",
  "geo": "US",
  "hl": "en"
}
```

Localized market research:

```json
{
  "query": "electric vehicles",
  "geo": "DE",
  "hl": "de"
}
```

## Development

```bash
npm install
npm test
```

## Run Locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"query":"tesla","geo":"US","hl":"en"}'
```

## Scrappa API

This actor is a thin wrapper around Scrappa's Google Trends API. It calls:

- `GET https://scrappa.co/api/google-trends/autocomplete`

For higher-volume workloads, direct Scrappa API access avoids Apify run overhead while keeping the same underlying data source.
