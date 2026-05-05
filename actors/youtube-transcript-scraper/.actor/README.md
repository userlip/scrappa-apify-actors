# YouTube Transcript Scraper

Extract transcript segments and full transcript text for a YouTube video by video ID. The Actor supports language selection, translated transcripts when YouTube exposes them, and locale hints for transcript metadata.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube video ID. |
| `language` | string | No | Preferred transcript or translation language code, such as `en`, `es`, `de`, or `fr`. |
| `hl` | string | No | Two-letter YouTube interface language code. |
| `gl` | string | No | Two-letter country code for YouTube localization. |
| `debug` | boolean | No | Include Scrappa debug metadata when available. |

## Example Input

```json
{
  "id": "dQw4w9WgXcQ",
  "language": "en",
  "hl": "en",
  "gl": "US"
}
```

## Output

The Actor stores one dataset item per video. Each item includes the full transcript text, timed transcript segments, returned language metadata, and language availability arrays. The full Scrappa response is also stored in the key-value store as `OUTPUT`.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
