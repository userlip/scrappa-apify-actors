# TrustedShops Shop Profile Scraper

Fetch TrustedShops merchant profile data by TSID or profile URL through Scrappa. Use this Actor after TrustedShops Search discovers merchant IDs and before collecting full review feeds.

## What It Extracts

- TrustedShops TSID
- Merchant display name
- Merchant website URL
- TrustedShops profile URL
- Language and target market fields when TrustedShops exposes them
- Rating and review count summary
- Certification status
- Categories and profile metadata

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tsids` | array | No | Recommended batch input. Each TSID should be a 33-character TrustedShops ID. |
| `urls` | array | No | TrustedShops profile URLs. The Actor extracts TSIDs when possible. |
| `tsid` | string | No | Single TSID convenience field. Prefer `tsids` for normal usage. |
| `url` | string | No | Single profile URL convenience field. Prefer `urls` for normal usage. |
| `include_raw_response` | boolean | No | Include the full Scrappa response in each dataset item. Default `false`. |

Provide at least one TSID or profile URL. Batch multiple merchants in one run to reduce Apify run overhead.

## Example Input

```json
{
  "tsids": [
    "XFB15FFBDE1DEE7A55D292A7D48598A6A"
  ],
  "urls": [
    "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
  ],
  "include_raw_response": false
}
```

## Output

Each successful shop profile is saved as one dataset item:

```json
{
  "tsid": "XFB15FFBDE1DEE7A55D292A7D48598A6A",
  "requested_tsid": "XFB15FFBDE1DEE7A55D292A7D48598A6A",
  "name": "Example Shop",
  "url": "https://www.example-shop.de",
  "profile_url": "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html",
  "language": "de",
  "target_market": "DEU",
  "rating": 4.8,
  "review_count": 173960,
  "certified": true,
  "category_names": "Fashion, Shoes",
  "source_url": null
}
```

The `OUTPUT` key-value-store record contains a run summary and any failed inputs. Failed or invalid inputs are not charged as shop profile results.

## Workflow

1. Use **TrustedShops Search Scraper** to discover merchants and TSIDs from a shop, brand, or domain query.
2. Use **TrustedShops Shop Profile Scraper** to enrich each TSID with merchant-level profile data.
3. Use a TrustedShops Reviews Actor or Scrappa's TrustedShops Reviews API to collect review-level records for selected shops.

For higher-volume workflows or direct API integration, call Scrappa directly at `https://scrappa.co/api/trustedshops/shop/{tsid}`.
