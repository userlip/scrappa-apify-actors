# Google Maps Autocomplete

Get autocomplete suggestions from Google Maps. Perfect for location discovery, address validation, and search interface building.

## Features

- **Real-time Suggestions** - Get predictions as user types
- **Multiple Types** - Places, addresses, businesses, and more
- **Coordinates** - Location data for each suggestion
- **Quick Lookup** - Fast response times
- **Address Validation** - Verify user input

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Partial search term (e.g., 'time sq', 'starbucks new') |

## Output

Array of suggestion objects with:
- `main_text` - The suggestion text
- `type` - Type (place, address, business, etc.)
- `latitude` / `longitude` - Coordinates
- `country` - Country code
- `google_id` / `place_id` - IDs for further queries

## Example Usage

1. User starts typing: "new yo"
2. Call autocomplete with: `{"query": "new yo"}`
3. Get suggestions for "New York", "New York Coffee", etc.
4. Use returned `business_id` with Details or Reviews actors

## Pricing

$0.30 per 1,000 results. No API keys required.
