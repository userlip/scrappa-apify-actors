# Baidu Autocomplete Scraper

Expand seed phrases into Baidu autocomplete suggestions with rank, suggestion text, and source markers. Batch searches to compare related keyword ideas across topics.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Rank of this suggestion in Baidu autocomplete results. |
| `query` | String | Query text returned for the suggestion entry. |
| `value` | String | Suggested search phrase users may enter next. |
| `type` | String | Baidu suggestion type label. |
| `is_direct` | Boolean | True when Baidu marks the phrase as a direct query match. |
| `sa` | String | Baidu source marker associated with the suggestion. |
| `q` | String | Normalized suggestion query text returned by Baidu. |
| `no_ai_sug` | Boolean | True when Baidu marks the phrase as excluded from AI suggestions. |
| `input_query` | String | Baidu seed phrase submitted to retrieve autocomplete suggestions. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO teams can discover related phrases people enter in Baidu search.
- Product teams can map localized wording around products and services.
- Content planners can compare autocomplete suggestions across multiple seeds.

## How to use

1. Add one seed phrase to `queries` for each lookup.
2. Set `maxResults` to limit the number of suggestion rows saved.
3. Review each suggestion with its source phrase and rank.

```json
{
  "queries": [
    {
      "query": "iphone"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Seed phrases to expand with Baidu autocomplete. |
| `queries[].query` | string | Yes per entry | Search Query sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "query": "iphone 17",
  "value": "iphone 17",
  "type": "suggestion",
  "is_direct": false,
  "sa": "s_1",
  "q": "iphone 17",
  "no_ai_sug": false,
  "input_query": "iphone",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~baidu-autocomplete-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are suggestions translated to English?

No. Baidu returns suggestions in the language and market associated with the query.

## Related Scrappa Actors

- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
