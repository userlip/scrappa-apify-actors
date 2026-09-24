# Website Content Extractor Scraper

Extract website content from one or more URLs with Scrappa's Web Scraper API.

This actor is a Rust wrapper around `GET https://scrappa.co/api/web-scraper`. It writes one Apify dataset item per URL and a run summary to the default key-value store's `OUTPUT` record. Use `response_type=json` for structured URL to JSON extraction or `response_type=markdown` for HTML to Markdown output.

## Example Input

```json
{
  "urls": ["https://example.com"],
  "include_html": false,
  "response_type": "json"
}
```

## Example Output

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
  "languages_detected": ["en"]
}
```

Batch URLs in one run whenever possible. For high-volume Web Scraper API usage, use Scrappa direct API access at `https://scrappa.co/api/web-scraper`.

The single-URL `url` input remains supported for existing prefills. Successful results use the `url-result` pay-per-event charge; per-URL API failures are written as uncharged error rows so the rest of the batch can continue.
