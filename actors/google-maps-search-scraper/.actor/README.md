# Google Maps Search

Search for businesses on Google Maps at scale. Find restaurants, services, shops, and more with detailed information.

## Features

- **Business Discovery** - Search for any type of business or service
- **Complete Data** - Ratings, review counts, address, phone, website
- **Photos & Hours** - Opening hours and sample photos for each result
- **Multiple Results** - Returns all matching businesses from search
- **No Location Limit** - Search globally for any query
- **Rich Information** - Price level, business type, status

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Search query (e.g., "restaurants in NYC") |
| `language` | string | No | Language code (e.g., 'en', 'de', default: 'en') |
| `country` | string | No | Country code (e.g., 'us', 'uk') |

## Output

Multiple business records with ratings, reviews, address, contact info, and more.

## Pricing

$0.30 per 1,000 results. No API keys required.
