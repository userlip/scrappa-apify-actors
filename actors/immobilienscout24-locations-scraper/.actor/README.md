# ImmobilienScout24 Locations Scraper

Find ImmobilienScout24 city and district suggestions with the location codes used in property searches. Enter a city or district to get matching ImmobilienScout24 location suggestions.

## What data can you extract?

Location names and codes are suggestions used to narrow ImmobilienScout24 property searches.

| Field | Type | Description |
| --- | --- | --- |
| `geocode` | text | ImmobilienScout24 location code for the matched city or district; null when the suggestion has no location code. |
| `name` | text | Name of the property listing, as shown by ImmobilienScout24; null when no name is published. |
| `type` | text | Category assigned to the property listing by ImmobilienScout24; null when ImmobilienScout24 does not provide the value. |
| `source_query` | text | Source query shown for the property listing by ImmobilienScout24, in the format used by the source; null when it is omitted. |
| `is_cached` | boolean | Whether cached place information was used; false is a reported value, while null means ImmobilienScout24 provided no flag. |

## Use cases

- Agents can compare asking prices, room counts and floor area in a target market.
- Researchers can review homes by location before building a market snapshot.
- Search teams can collect listing details for a property shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by ImmobilienScout24.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "Berlin"
  ],
  "limit": 10
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Batch-first list of up to 100 city, district, or postal-code queries. Constraints: minimum 1 items; maximum 100 items. |
| `query` | string | No | Use this for older integrations that send one query. The queries list takes precedence when both are provided. |
| `limit` | integer | No | Maximum location matches requested from Scrappa for each query. Constraints: minimum 1; maximum 20. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Northstar Market Labs",
  "geocode": "1276006",
  "type": "Video",
  "source_query": "Seattle, WA",
  "is_cached": false
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved property record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~immobilienscout24-locations-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I find a location code for an ImmobilienScout24 search?

Enter a city or district in the location query. Use the returned location code with ImmobilienScout24 Search to target that area.

## Related Scrappa Actors

- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)
