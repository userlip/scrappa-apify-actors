# Website Content Extractor Scraper

Extract website content from one URL or a batch of URLs with Scrappa's Web Scraper API. This actor is a thin Apify marketplace wrapper around `GET https://scrappa.co/api/web-scraper`; the scraping runs on Scrappa infrastructure, while Apify handles inputs, runs, and dataset output.

Use it when you need a Website Content Extractor, Web Scraper API, URL to JSON pipeline, or HTML to Markdown conversion without building your own proxy and parsing stack.

## What You Get

- URL to JSON extraction with title, meta description, keywords, favicon, social links, body text, detected languages, links, emails, phone numbers, and images
- Optional raw HTML with `include_html=true`
- HTML to Markdown output with `response_type=markdown`
- One Apify dataset item per processed URL
- Per-URL success/error metadata so a failed target URL does not hide successful URLs in the same batch
- `site_status_code` kept separate from actor/API failures, so target-site 404/500 responses can be handled downstream

## Input

Batch input is preferred because it keeps Apify run overhead low:

```json
{
  "urls": [
    "https://example.com",
    "https://www.iana.org/domains/reserved"
  ],
  "include_html": false,
  "response_type": "json"
}
```

Backward-compatible single URL input is also supported:

```json
{
  "url": "https://example.com",
  "response_type": "markdown"
}
```

## Output

JSON mode writes one dataset item per URL:

```json
{
  "success": true,
  "input_url": "https://example.com",
  "request_url": "https://example.com",
  "response_type": "json",
  "include_html": false,
  "site_status_code": 200,
  "url": "https://example.com",
  "final_url": "https://example.com",
  "title": "Example Domain",
  "description": "This domain is for use in illustrative examples.",
  "body_text": "Example Domain This domain is for use in illustrative examples in documents.",
  "links_count": 1,
  "emails_count": 0,
  "phone_numbers_count": 0,
  "images_count": 0,
  "languages_detected": ["en"],
  "data": {
    "title": "Example Domain",
    "links": ["https://www.iana.org/domains/example"]
  }
}
```

Markdown mode normalizes Scrappa's plain-text response into a dataset item:

```json
{
  "success": true,
  "input_url": "https://example.com",
  "request_url": "https://example.com",
  "response_type": "markdown",
  "include_html": false,
  "url": "https://example.com",
  "markdown": "# Example Domain\n\nThis domain is for use in illustrative examples...",
  "markdown_length": 89
}
```

If one URL fails validation or Scrappa returns an infrastructure error, the actor writes an error row for that URL and continues with the rest of the batch.

## High-Volume Use

For high-volume website content extraction or direct Web Scraper API access, call Scrappa directly at `https://scrappa.co/api/web-scraper`. This Apify actor is optimized for marketplace workflows and batched URL extraction, not for running a separate Apify job per page.
