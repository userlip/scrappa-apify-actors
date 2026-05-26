# Google Patents Details Scraper

Enrich known Google Patents publication IDs and URLs with full patent metadata, abstracts, inventors, assignees, dates, citations, and links.

This actor is for detail enrichment after you already have patent IDs. Use `google-patents-search-scraper` when you need keyword, assignee, inventor, country, status, type, or date-filtered discovery.

The actor accepts single or batch inputs and returns one dataset item per patent, including `input_patent_id`, `normalized_patent_id`, `success`, detail fields, and structured error fields for failed IDs.

Recommended paid pricing: **$0.20 per 1,000 dataset items**.
