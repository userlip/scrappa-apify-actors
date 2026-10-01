# Startpage Search Scraper

Find Startpage results with page titles, descriptions, domains and source links. Submit several phrases in one run and use only the locale and filters listed in the input.

## What data can you extract?

Titles, snippets and displayed links follow Startpage results for the selected phrase and region.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Startpage web search result list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the web search result, as shown by Startpage; null when no title is published. |
| `description` | text | Description text from Startpage for this web search result; null when the source has no text to show. |
| `url` | link | Source page url for this web search result on Startpage; null when the source does not provide a URL. |
| `domain` | text | Domain shown for the web search result by Startpage, in the format used by the source; null when it is omitted. |
| `source` | text | Source or language label shown for the web search result by Startpage, in the format used by the source; null when it is omitted. |
| `query` | text | Query shown for the web search result by Startpage, in the format used by the source; null when it is omitted. |
| `request_language` | text | Language code passed to Startpage. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Startpage; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_safe_search` | number | Safe-search setting passed to Startpage. This input value is copied into the output row; null when it was not supplied. |
| `total_results` | number | Number of total results shown by Startpage, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- SEO teams can check which pages and domains appear for a query.
- Communications teams can monitor snippets and links for a brand or topic.
- Researchers can compare titles and domains across locales or repeat searches.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a topic or search phrase, then adjust the locale, page or time range fields that this Actor supports.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    {
      "query": "privacy tools",
      "language": "english",
      "page": 0,
      "safe_search": true
    }
  ],
  "max_results_per_query": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of object | Yes | Search requests to run in one Actor run. Constraints: minimum 1 items; maximum 100 items. |
| `max_results_per_query` | integer | No | Maximum organic results to save for each query. Constraints: minimum 1; maximum 100. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "A practical guide to independent neighborhood shops",
  "description": "A guide to choosing containers, light and watering schedules for a compact balcony herb garden.",
  "url": "https://listings.example.com/record/731-alder-way",
  "domain": "northstar.example",
  "source": "Google Search",
  "query": "weekend markets in Seattle",
  "total_results": 56
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved source match counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~startpage-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Startpage Search run several queries together?

Yes. Add multiple phrases to the batch input and set the supported locale or filters. Each result row retains its search context.

## Related Scrappa Actors

- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Search Results Scraper](https://apify.com/thescrappa/scrappa-google-search)
