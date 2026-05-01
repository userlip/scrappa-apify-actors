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
| `hl` | string | No | Language code (e.g., `en`, `de`, `en-US`; default: `en`) |
| `gl` | string | No | Country or region code (e.g., `us`, `uk`, `de`) |
| `debug` | boolean | No | Enable Scrappa debug logging for admin troubleshooting |
| `use_cache` | boolean | No | Use cached results when available |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds |

## Output

The actor returns one dataset item per business. Fields can include:

| Field | Description |
|-------|-------------|
| `name` | Business name |
| `type`, `subtypes` | Primary and secondary business categories |
| `rating`, `review_count` | Google rating and review count |
| `price_level`, `price_level_text` | Price metadata when available |
| `full_address`, `district`, `timezone` | Address and local context |
| `latitude`, `longitude` | Business coordinates |
| `phone_numbers`, `website`, `domain` | Contact and website details |
| `business_id`, `place_id`, `google_mid` | Google and Scrappa identifiers |
| `owner_id`, `owner_name`, `owner_link` | Owner metadata when available |
| `order_link` | Ordering URL when available |
| `short_description`, `full_description` | Business descriptions |
| `current_status` | Current open or operating status |
| `photos_sample` | Sample photo objects with photo URLs and coordinates |
| `opening_hours` | Opening hour objects by day, including special days |

## Pricing

$0.30 per 1,000 results. No API keys required.
