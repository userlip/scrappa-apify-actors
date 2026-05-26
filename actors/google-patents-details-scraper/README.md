# Google Patents Details Scraper

Enrich Google Patents publication IDs and URLs through Scrappa and export one structured dataset item per patent. Use it when you already have patent IDs from search results, prior-art lists, competitor monitoring, IP due diligence, assignee research, R&D review, or internal patent workflows and need full metadata without running a broad search again.

This actor is a focused companion to `google-patents-search-scraper`: search discovers patent results, details enriches known publication IDs or Google Patents URLs.

## Pricing

This actor is intended for paid per-result monetization. Recommended marketplace pricing is **$0.20 per 1,000 dataset items** so users pay for returned patent detail records and can batch many IDs in one Apify run.

## Input

Provide any mix of a single patent ID, batch patent IDs, a single Google Patents URL, or batch URLs. Duplicate normalized IDs are fetched once.

```json
{
  "patent_id": "US9789384B1",
  "patent_ids": ["EP3892147A1", "WO2020123456A1"],
  "urls": [
    "https://patents.google.com/patent/US9789384B1"
  ]
}
```

### Fields

- `patent_id` - Single publication ID or full Google Patents ID, such as `US9789384B1` or `patent/US9789384B1/en`.
- `patent_ids` - Array of publication IDs or full Google Patents IDs for batch enrichment.
- `url` - Single Google Patents URL.
- `urls` - Array of Google Patents URLs for batch enrichment.

Accepted URL shape: `https://patents.google.com/patent/US9789384B1`. The actor normalizes URL inputs to the Scrappa `patent/US9789384B1/en` format before calling `/google-patents/details`.

## Output

Each dataset item represents one input patent ID or URL:

```json
{
  "success": true,
  "input_patent_id": "US9789384B1",
  "normalized_patent_id": "patent/US9789384B1/en",
  "patent_id": "patent/US9789384B1/en",
  "publication_number": "US9789384B1",
  "patent_page": "https://patents.google.com/patent/US9789384B1",
  "title": "Self-balancing board having a suspension interface",
  "abstract": "Patent abstract text...",
  "inventors": ["Example Inventor"],
  "assignees": {
    "original": ["Example Assignee"]
  },
  "dates": {
    "priority": "2013-03-15",
    "filing": "2014-03-14",
    "publication": "2015-09-17",
    "grant": "2017-10-17"
  },
  "links": {
    "pdf": "https://patentimages.storage.googleapis.com/..."
  },
  "citations": {
    "patent": []
  },
  "inventor_count": 1,
  "assignee_count": 1,
  "citation_count": 0,
  "cached": false,
  "response_time_ms": 1234
}
```

Failed patents are also pushed as dataset items so batch runs keep one output row per requested patent:

```json
{
  "success": false,
  "input_patent_id": "US9999999999",
  "normalized_patent_id": "patent/US9999999999/en",
  "error": "Scrappa API error (404): Patent 'patent/US9999999999/en' not found",
  "status_code": 404
}
```

For a single-patent run, the same item is also stored in the key-value store as `OUTPUT`.

## Notes

- Batch IDs or URLs in one run to keep Apify run overhead low while still receiving one charged dataset item per patent.
- Use the search actor first when you need discovery by keywords, inventors, assignees, country, status, type, or date filters.
- For higher-volume direct API usage, use Scrappa's Google Patents API.
