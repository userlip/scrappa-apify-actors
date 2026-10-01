# Website Content Extractor

Extract page titles, descriptions, readable text and links from public web pages. Add one or more public page URLs and choose whether to include HTML in the extracted content.

## What data can you extract?

Extracted text and page metadata depend on what the submitted public page exposes.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means the submitted website provided no flag. |
| `input_url` | link | Source page url passed to the submitted website. This input value is copied into the output row; null when it was not supplied. |
| `url` | link | Source page url for this web page on the submitted website; null when the source does not provide a URL. |
| `final_url` | link | Final page url for this web page on the submitted website; null when the source does not provide a URL. |
| `response_type` | text | Response type shown for the web page by the submitted website; null when the submitted website does not provide the value. |
| `include_html` | boolean | Whether HTML was requested for the page; false is a reported value, while null means the submitted website provided no flag. |
| `site_status_code` | number | Site status code shown for the web page by the submitted website, in the format used by the source; null when it is omitted. |
| `title` | text | Title of the web page, as shown by the submitted website; null when no title is published. |
| `description` | text | Description text from the submitted website for this web page; null when the source has no text to show. |
| `body_text` | text | Readable page text from the submitted website for this web page; null when the source has no text to show. |
| `links_count` | number | Number of links shown by the submitted website, as a whole number; zero is possible, and null means no count was reported. |
| `emails_count` | number | Number of emails shown by the submitted website, as a whole number; zero is possible, and null means no count was reported. |
| `phone_numbers_count` | number | Number of phone numbers shown by the submitted website, as a whole number; zero is possible, and null means no count was reported. |
| `images_count` | number | Number of images shown by the submitted website, as a whole number; zero is possible, and null means no count was reported. |
| `languages_detected` | array | Languages detected in the submitted page or text from the submitted website; an empty list when no entries are available. |
| `markdown` | text | Page content in markdown from the submitted website for this web page; null when the source has no text to show. |
| `markdown_length` | number | Markdown length shown for the web page by the submitted website, in the format used by the source; null when it is omitted. |
| `error` | text | Diagnostic text for the the submitted website lookup; null when the request completes without an error. |
| `error_type` | text | Diagnostic text for the the submitted website lookup; null when the request completes without an error. |
| `error_code` | text | Diagnostic text for the the submitted website lookup; null when the request completes without an error. |
| `status_code` | number | Http status code shown for the web page by the submitted website, in the format used by the source; null when it is omitted. |

## Use cases

- SEO teams can audit page titles, descriptions, links and readable text across a site.
- Editors can review extracted copy before updating an article or landing page.
- Researchers can collect page content and metadata from public URLs.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by the website URL you provide.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "urls": [
    "https://example.com"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array | No | Preferred batch input. Enter one or more page URLs to extract in a single actor run. Constraints: minimum 1 items; maximum 20 items. |
| `url` | string | No | Backward-compatible single URL input. Prefer URLs for batching. |
| `include_html` | boolean | No | Include raw page HTML in JSON responses. Ignored when Response Type is Markdown. |
| `response_type` | string | No | Choose JSON for structured extraction fields, or Markdown for clean page content. Constraints: allowed values: json, markdown. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "languages_detected": [
    "en",
    "es"
  ],
  "title": "A practical guide to independent neighborhood shops",
  "description": "Northstar Market Labs helps local retailers plan inventory with a clear view of seasonal demand.",
  "url": "https://listings.example.com/record/731-alder-way",
  "final_url": "https://source.example.com/record/market-guide",
  "response_type": "text",
  "include_html": false,
  "site_status_code": 7.3
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~website-content-extractor-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Website Content Extractor process several pages at once?

Yes. Submit page URLs in `urls` or a single URL in `url`. `include_html` and `response_type` control the optional page content returned.

## Related Scrappa Actors

- [Domain Availability Checker](https://apify.com/thescrappa/domain-availability-checker)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
