# Google AI Overview Scraper

Submit search questions and receive available AI Overview content, text blocks, sections, displayed query, and formatted answer text. Coverage depends on query and location.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `search_information` | Object | Search context, including the query Google displays with the response. |
| `ai_overview` | Object | Structured Google AI Overview answer blocks and referenced sources when available. |
| `text` | String | Plain-text version of the Google AI Overview answer. |
| `markdown` | String | Markdown version of the answer and its section structure. |
| `service_used` | String | Search service label returned with the AI Overview response. |
| `query_displayed` | String | Search phrase Google shows for this AI Overview response. |
| `input_query` | String | Search phrase submitted to Google for this answer lookup. |
| `scraped_at` | String | UTC date and time when this Google AI Overview was collected. |

## Use cases

- AI search teams can compare answer coverage across a query set.
- Content strategists can inspect how Google sections and phrases generated answers.
- Product researchers can track where Google AI Overviews appear in search.

## How to use

1. Add one question to `queries` for each overview to capture.
2. Set language and country for localized search context.
3. Read one structured response item per query.

```json
{
  "queries": [
    {
      "query": "how does solar power work"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Questions or phrases to check for Google AI Overview content. |
| `queries[].query` | string | Yes per entry | Search query sent to Google AI Mode. |
| `queries[].hl` | string | No | Interface language code \(2 letters, e.g. en, de\). |
| `queries[].gl` | string | No | Country code \(2 letters, e.g. us, de\). |
| `queries[].google_domain` | string | No | Google domain \(e.g. google.de\). |
| `queries[].uule` | string | No | Encoded location, passed through when provided. |
| `hl` | string | No | Interface language code \(2 letters, e.g. en, de\). |
| `gl` | string | No | Country code \(2 letters, e.g. us, de\). |
| `google_domain` | string | No | Google domain \(e.g. google.de\). |
| `uule` | string | No | Encoded location, passed through when provided. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "search_information": {
    "query_displayed": "how do heat pumps work",
    "search_url": "https://www.google.com/search?q=how+do+heat+pumps+work"
  },
  "ai_overview": {
    "text_blocks": [
      {
        "type": "paragraph",
        "snippet": "Heat pumps transfer heat between indoor and outdoor air, using electricity to provide efficient heating and cooling.",
        "snippet_markdown": "Heat pumps transfer heat between indoor and outdoor air, using electricity to provide efficient heating and cooling."
      }
    ],
    "sections": [
      {
        "title": "Basic operation",
        "places": [
          {
            "name": "U.S. Department of Energy",
            "address": "1000 Independence Avenue SW, Washington, DC"
          }
        ]
      }
    ]
  },
  "text": "Heat pumps transfer heat between indoor and outdoor air, using electricity to provide efficient heating and cooling.",
  "markdown": "Heat pumps transfer heat between indoor and outdoor air, using electricity to provide efficient heating and cooling.",
  "service_used": "google",
  "query_displayed": "how do heat pumps work",
  "input_query": "how does solar power work",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-ai-overview-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Will every query return an AI Overview?

No. Google may omit an overview for some searches. A successful response can contain no overview text.

## Related Scrappa Actors

- [Google Scholar Scraper](https://apify.com/thescrappa/google-scholar-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)
