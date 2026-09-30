# Google AI Overview Scraper

Fetch the Google AI Overview and answer text for a search query.

## Data you get

- **query_displayed**: Search phrase displayed by Google.
- **text**: Review or result text.
- **markdown**: Formatted AI Overview answer text.
- **ai_overview**: Ai overview returned for this Google AI Overview Scraper result.
- **service_used**: Scrappa service used for the response.

## Use cases

- Search answer monitoring
- Research summaries
- SEO and answer-engine analysis

## How to use

Add one or more entries to **queries**. Each entry maps its **query** value to the Scrappa **query** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

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
        "snippet": "A synthetic explanation of heat pumps.",
        "snippet_markdown": "A synthetic explanation of heat pumps."
      }
    ],
    "sections": [
      {
        "title": "Basic operation",
        "places": []
      }
    ]
  },
  "text": "A synthetic explanation of heat pumps.",
  "markdown": "A synthetic explanation of heat pumps.",
  "service_used": "google",
  "input_query": "how does solar power work",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_query** and **scraped_at** for traceability.

## Related Actors

- [Google Scholar Scraper](https://apify.com/thescrappa/google-scholar-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)

## Search terms

`Google AI Overview Scraper`, `query_displayed`, `text`, `markdown`, `/search-ai-overview API`
