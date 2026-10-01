# Google Search Scraper

Find Google Search results with page titles, displayed domains, snippets and links. Use the phrase, location and locale options to shape a Google results page for your query.

## What data can you extract?

Titles, snippets and displayed links follow the search results for the selected phrase and region.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Search web search result list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the web search result, as shown by Google Search; null when no title is published. |
| `link` | link | Result link for this web search result on Google Search; null when the source does not provide a URL. |
| `displayed_link` | text | Displayed domain shown for the web search result by Google Search, in the format used by the source; null when it is omitted. |
| `snippet` | text | Search snippet from Google Search for this web search result; null when the source has no text to show. |
| `source` | text | Source or language label shown for the web search result by Google Search, in the format used by the source; null when it is omitted. |

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
  "query": "best restaurants in new york"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | The search term or phrase to look up on Google |
| `location` | string | No | Geographic location for localized results (e.g., 'New York, NY, USA', 'London, UK') |
| `gl` | string | No | Two-letter country code for Google's country service (e.g., 'us', 'uk', 'de', 'fr') |
| `hl` | string | No | Two-letter language code for the interface (e.g., 'en', 'de', 'es', 'fr') |
| `google_domain` | string | No | Google domain to query (e.g., 'google.com', 'google.de', 'google.co.uk') |
| `start` | integer | No | Result offset for pagination. Use 0 for first page, 10 for second, 20 for third, etc. Constraints: minimum 0. |
| `amount` | integer | No | How many results to return per request (1-100) Constraints: minimum 1; maximum 100. |
| `safe` | string | No | Filter explicit content from results Constraints: allowed values: off, active. |
| `tbs` | string | No | Filter results by time. Examples: 'qdr:h' (past hour), 'qdr:d' (past day), 'qdr:w' (past week), 'qdr:m' (past month), 'qdr:y' (past year) |
| `tbm` | string | No | Type of Google search to perform Constraints: allowed values: , nws, vid, isch, shop. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Seattle neighborhood market guide",
  "link": "https://search.example.com/results/market-guide",
  "displayed_link": "northstar.example/market-guide",
  "snippet": "A guide to choosing containers, light and watering schedules for a compact balcony herb garden.",
  "source": "Google Search"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How can I target a Google Search market?

Set `query` to the phrase and use `gl`, `hl` or `google_domain` for supported market and language settings. `location` can further localize a search.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Search Results Scraper](https://apify.com/thescrappa/scrappa-google-search)
- [Startpage Search Scraper](https://apify.com/thescrappa/startpage-search-scraper)
