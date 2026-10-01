# TrustedShops Shop Profile Scraper

Look up a Trusted Shops profile with shop details, rating and certification status. Submit a shop ID or profile URL, with batch input available for multiple shops.

## What data can you extract?

Ratings and review details follow public Trusted Shops profiles; some shops have no review text or reply.

| Field | Type | Description |
| --- | --- | --- |
| `tsid` | text | Trusted Shops ID for the business profile, assigned by Trusted Shops; null when the source does not expose it. |
| `requested_tsid` | text | requested tsid for the business profile, assigned by Trusted Shops; null when the source does not expose it. |
| `name` | text | Name of the business profile, as shown by Trusted Shops; null when no name is published. |
| `url` | link | Source page url for this business profile on Trusted Shops; null when the source does not provide a URL. |
| `profile_url` | link | Profile url for this business profile on Trusted Shops; null when the source does not provide a URL. |
| `language` | text | Language code or language name used for this text; null when Trusted Shops does not provide the value. |
| `target_market` | text | Target market shown for the business profile by Trusted Shops, in the format used by the source; null when it is omitted. |
| `rating` | number | Rating for this business profile, on Trusted Shops’ 1-to-5 rating scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Trusted Shops, as a whole number; zero is possible, and null means no count was reported. |
| `certified` | boolean | Whether the shop has a certification; false is a reported value, while null means Trusted Shops provided no flag. |
| `category_names` | text | Category names shown for the business profile by Trusted Shops, in the format used by the source; null when it is omitted. |
| `category_ids` | text | category ids for the business profile, assigned by Trusted Shops; null when the source does not expose it. |
| `source_url` | link | Source page url for this business profile on Trusted Shops; null when the source does not provide a URL. |
| `profile_metadata` | object | Shop profile metadata with the source and profile version details from Trusted Shops; null when the source provides no details. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `tsids` and use the identifier or URL format required by Trusted Shops.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "tsids": [
    "XFB15FFBDE1DEE7A55D292A7D48598A6A"
  ],
  "urls": [
    "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tsids` | array of string | No | Recommended. Process many TrustedShops IDs in one Apify run. Each successful TSID produces one charged dataset item. |
| `urls` | array of string | No | TrustedShops profile URLs. The actor extracts TSIDs from URLs when possible, then fetches the shop profile. |
| `tsid` | string | No | Single TrustedShops ID. Prefer TSIDs for batch usage. |
| `url` | string | No | Single TrustedShops profile URL. Prefer URLs for batch usage. |
| `include_raw_response` | boolean | No | Include the full Scrappa TrustedShops shop profile response in each dataset item for debugging or custom field mapping. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "profile_metadata": {
    "apiVersion": "2.4",
    "source": "public shop profile"
  },
  "name": "Northstar Market Labs",
  "rating": 4.7,
  "review_count": 184,
  "url": "https://listings.example.com/record/731-alder-way",
  "tsid": "TS-7281945",
  "requested_tsid": "TS-7281945",
  "profile_url": "https://profiles.example.com/northstar-market-labs"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each completed profile or detail lookup counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustedshops-shop-profile-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I look up several Trusted Shops profiles?

Use `tsids` or `urls` for a batch, or `tsid` or `url` for one shop. The profile fields depend on what Trusted Shops makes public.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)
