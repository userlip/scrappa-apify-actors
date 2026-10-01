# Google Trends Autocomplete Scraper

Find topic and query suggestions from Google Trends before building a search list. Enter a topic or phrase and choose a region and language to guide related suggestions.

## What data can you extract?

Suggestions are related phrases or topics from Google Trends; interest scores are relative when included.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Trends topic suggestion list, as a whole number; null when the source does not supply one. |
| `suggestion` | text | Suggested phrase shown for the topic suggestion by Google Trends, in the format used by the source; null when it is omitted. |
| `type` | text | Category assigned to the topic suggestion by Google Trends; null when Google Trends does not provide the value. |
| `source_keyword` | text | Source keyword shown for the topic suggestion by Google Trends, in the format used by the source; null when it is omitted. |
| `request_geo` | text | Geographic target passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Trends; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
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
  "query": "tesla"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Partial keyword or phrase to expand with Google Trends autocomplete suggestions. |
| `q` | string | No | Alias for query when reusing direct Scrappa API inputs. Either Search Query or this alias is required. |
| `geo` | string | No | Geographic location code, such as US, GB, DE, or Worldwide. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "suggestion": "weekend farmers markets",
  "type": "Video",
  "source_keyword": "weekend markets",
  "response_time_ms": 348
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-trends-autocomplete-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What does Google Trends Autocomplete return for a topic?

Enter a phrase in `query` or `q`, and optionally set `geo` and `hl`. The response contains suggested topics related to the source query.

## Related Scrappa Actors

- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
