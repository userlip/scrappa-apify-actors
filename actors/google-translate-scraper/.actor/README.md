# Google Translate Scraper

Translate text between Google Translate languages and export one dataset item per translated text. This actor wraps Scrappa's `/api/google-translate` endpoint and keeps the translation work on Scrappa infrastructure.

## What you get

- Batch translation through `items[]` in one Apify run
- Single-item compatibility with top-level `text`, `source`, and `target`
- One dataset item per successful translation result
- Clear per-item error rows when an individual translation fails but the batch can continue

## Input

```json
{
  "items": [
    { "text": "Good morning", "source": "en", "target": "de" },
    { "text": "How are you?", "source": "en", "target": "es" }
  ]
}
```

For a single translation:

```json
{
  "text": "Good morning",
  "source": "en",
  "target": "de"
}
```

## Output

Each dataset item represents one translation request:

```json
{
  "success": true,
  "index": 0,
  "text": "Good morning",
  "translated_text": "Guten Morgen",
  "source": "en",
  "target": "de",
  "error": null,
  "status_code": null
}
```

If an individual item fails after Scrappa retries, the actor writes an uncharged error row and continues with the rest of the batch:

```json
{
  "success": false,
  "index": 1,
  "text": "How are you?",
  "translated_text": null,
  "source": "en",
  "target": "es",
  "error": "Translation service temporarily unavailable. Please retry.",
  "status_code": 503
}
```

## Notes

The actor only accepts `text`, `source`, and `target`. Scrappa's admin-only `append` and `html` parameters are intentionally not exposed.

The dataset is the primary output channel. For compatibility, single-item runs also write that translation item to `OUTPUT`; batch runs write a compact summary to `OUTPUT`.

For higher-volume translation or direct API access, use Scrappa's Google Translate API at `https://scrappa.co/api/google-translate`.
