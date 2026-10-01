# Google Trends Related Queries Scraper

Explore related Google Trends queries and topics with their relative interest scores. Choose a region and time range to see which searches are associated with the selected topic.

## What data can you extract?

Related queries and topics reflect the selected region and period; interest values are relative scores, not raw search counts.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Trends related query list, as a whole number; null when the source does not supply one. |
| `result_kind` | text | Category assigned to the related query by Google Trends; null when Google Trends does not provide the value. |
| `type` | text | Category assigned to the related query by Google Trends; null when Google Trends does not provide the value. |
| `query` | text | Query shown for the related query by Google Trends, in the format used by the source; null when it is omitted. |
| `topic` | text | Topic shown for the related query by Google Trends, in the format used by the source; null when it is omitted. |
| `topic_type` | text | Category assigned to the related query by Google Trends; null when Google Trends does not provide the value. |
| `value` | number | Source value shown for the related query by Google Trends, in the format used by the source; null when it is omitted. |
| `formatted_value` | text | Formatted value shown for the related query by Google Trends, in the format used by the source; null when it is omitted. |
| `link` | link | Result link for this related query on Google Trends; null when the source does not provide a URL. |
| `source_keyword` | text | Source keyword shown for the related query by Google Trends, in the format used by the source; null when it is omitted. |
| `request_geo` | text | Geographic target passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_time_range` | text | Time range passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Trends; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_search_type` | text | Search category passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `response_time_ms` | number | Response time for this Google Trends lookup, measured in milliseconds; null when no timing value was recorded. |

## Use cases

- SEO specialists can collect suggested phrases or related queries while building a keyword cluster.
- Content planners can use Google Trends wording to outline supporting pages around a subject.
- Researchers can compare related terms across regions before localizing a content plan.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a topic or search phrase, then adjust the locale, page or time range fields that this Actor supports.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "coffee"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | Keyword or phrase to expand with Google Trends related queries. |
| `q` | string | No | Alias for query when reusing direct Scrappa API inputs. Apify users should fill Search Query. |
| `geo` | string | No | Geographic location code, such as US, GB, DE, or Worldwide. |
| `time_range` | string | No | Time period for related query discovery. Constraints: allowed values: 1h, 4h, 1d, 7d, 30d, 90d, 1y, 5y, all. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |
| `search_type` | string | No | Google Trends vertical to analyze. Constraints: allowed values: web, images, news, youtube, shopping. |
| `include_autocomplete` | boolean | No | Also call the Google Trends autocomplete endpoint and include suggestions in OUTPUT. Dataset rows remain focused on related queries and topics. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "link": "https://search.example.com/results/market-guide",
  "result_kind": "query",
  "type": "Video",
  "query": "weekend markets in Seattle",
  "topic": "urban gardening",
  "topic_type": "Lifestyle",
  "value": 64,
  "formatted_value": "Formatted value for the search result on Google Trends"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 queries.

Each processed search or query is counted according to the rate shown above.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-trends-related-queries-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What is the difference between top and rising Google Trends queries?

The source labels related queries by result type and reports relative interest. Set `time_range`, `geo` and `search_type` to choose the comparison.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
