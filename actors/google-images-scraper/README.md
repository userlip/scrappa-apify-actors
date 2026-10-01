# Google Images Scraper

Find Google Images results with image links, titles, publishers and dimensions. Choose a region and optional image filters to make a visual search more specific.

## What data can you extract?

Image links and dimensions are shown when Google Images provides them for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Images image result list, as a whole number; null when the source does not supply one. |
| `thumbnail_url` | image | Thumbnail url for this image result on Google Images; null when the source does not provide a URL. |
| `image_url` | link | Image url for this image result on Google Images; null when the source does not provide a URL. |
| `title` | text | Title of the image result, as shown by Google Images; null when no title is published. |
| `source` | text | Source or language label shown for the image result by Google Images, in the format used by the source; null when it is omitted. |
| `source_url` | link | Source page url for this image result on Google Images; null when the source does not provide a URL. |
| `width` | number | Image width of the image shown by Google Images, in pixels; null when the source does not publish the dimension. |
| `height` | number | Image height of the image shown by Google Images, in pixels; null when the source does not publish the dimension. |
| `is_product` | boolean | Whether Google Images classifies the result as a product; false is a reported value, while null means Google Images provided no flag. |
| `request_q` | text | Search phrase passed to Google Images. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Google Images; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Images; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Images; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_imgsz` | text | Image size filter passed to Google Images. This input value is copied into the output row; null when it was not supplied. |
| `request_imgtype` | text | Image type filter passed to Google Images. This input value is copied into the output row; null when it was not supplied. |
| `request_imgcolor` | text | Image color filter passed to Google Images. This input value is copied into the output row; null when it was not supplied. |
| `request_imgar` | text | Image aspect-ratio filter passed to Google Images. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Designers can collect image links and source pages while building visual references.
- Editorial teams can check image publishers and dimensions for a topic.
- Researchers can compare image results across phrases and locales.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by Google Images.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "coffee"
  ],
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Recommended. Process many Google Images keyword searches in one Apify run so run startup and storage overhead are shared across image results. Constraints: minimum 1 items; maximum 50 items. |
| `q` | string | No | Backward-compatible single keyword or phrase to search in Google Images. Prefer Search Queries for normal usage, especially when running more than one keyword. |
| `page` | integer | No | Page number for pagination. Page 1 starts at the first image result. Constraints: minimum 1. |
| `hl` | string | No | Two-letter interface language code, such as en, de, es, or fr. |
| `gl` | string | No | Two-letter country code for localized Google Images results, such as us, gb, de, or jp. |
| `imgsz` | string | No | Filter by image size. Constraints: allowed values: large, medium, icon. |
| `imgtype` | string | No | Filter by image type. Constraints: allowed values: photo, clipart, lineart, gif, face. |
| `imgcolor` | string | No | Filter by dominant image color. Constraints: allowed values: color, gray, trans, red, orange, yellow, green, teal, blue, purple, pink, white, black, brown. |
| `imgar` | string | No | Filter by image aspect ratio. Constraints: allowed values: tall, square, wide. |
| `tbs` | string | No | Google tbs filter syntax, such as qdr:d for past day, qdr:w for past week, qdr:m for past month, or qdr:y for past year. |
| `safe` | string | No | Safe search filtering. Constraints: allowed values: active, off. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Seattle neighborhood market guide",
  "thumbnail_url": "https://images.example.com/video/market-morning-thumb.jpg",
  "image_url": "https://images.example.com/listings/vintage-coat-01.jpg",
  "source": "Google Search",
  "source_url": "https://source.example.com/articles/market-guide",
  "width": 1280,
  "height": 720,
  "is_product": false
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results; plus $0.00005 per Actor Start event.

The listed amount is charged once when a run starts.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-images-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Images filter results by color or size?

Use the image filters listed in Input, such as size, color or type when supported. Set a phrase and locale to find matching image results.

## Related Scrappa Actors

- [Google Videos Scraper](https://apify.com/thescrappa/google-videos-scraper)
- [YouTube Search Scraper](https://apify.com/thescrappa/youtube-api-search-data)
- [YouTube Video Comments Scraper](https://apify.com/thescrappa/youtube-api-video-comments)
- [YouTube Transcript Scraper](https://apify.com/thescrappa/youtube-transcript-scraper)
