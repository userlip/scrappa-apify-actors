# Scrappa Google Search Actor

Scrape Google Search results using the [Scrappa](https://scrappa.co) API.

## Features

- Organic search results with position, title, URL, and snippet
- Localized search by country and language
- Pagination support
- Safe search filtering
- Time-based filtering
- Full response stored in key-value store

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `apiKey` | string | ✅ | Your Scrappa API key |
| `query` | string | ✅ | Search query |
| `location` | string | | Location for localized results |
| `gl` | string | | Two-letter country code (e.g., 'us') |
| `hl` | string | | Two-letter language code (e.g., 'en') |
| `start` | integer | | Pagination offset |
| `amount` | integer | | Number of results (1-100) |

## Output

Organic results are pushed to the dataset:

```json
{
  "position": 1,
  "title": "Example Title",
  "link": "https://example.com",
  "snippet": "Example description...",
  "displayed_link": "example.com"
}
```

Full API response is stored in the key-value store under the `OUTPUT` key.

## Example

```json
{
  "apiKey": "your-api-key",
  "query": "web scraping tools",
  "gl": "us",
  "hl": "en",
  "amount": 20
}
```

## API Key

Get your API key at [https://scrappa.co](https://scrappa.co).
