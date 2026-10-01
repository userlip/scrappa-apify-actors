# Google Translate Scraper

Translate short text with Google Translate and keep the source phrase with its language pair. Send the source text and target language, with batch input for multiple phrases.

## What data can you extract?

Translations are generated for the language pair supplied in the input.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Google Translate provided no flag. |
| `index` | number | Index in the Google Translate translation list, as a whole number; null when the source does not supply one. |
| `text` | text | Original text supplied for translation; null when the input item contains no text. |
| `translated_text` | text | Text translated by Google Translate into the requested target language; null if the translation is unavailable. |
| `source` | text | Source language code for the original text, such as en; null when automatic detection is used. |
| `target` | text | Target language code requested for the translation, such as de; null when not supplied. |
| `error` | text | Diagnostic text for the Google Translate lookup; null when the request completes without an error. |
| `status_code` | number | Http status code shown for the translation by Google Translate, in the format used by the source; null when it is omitted. |

## Use cases

- Localization teams can compare source phrases with translated text across languages.
- Editors can review short translations before adding them to a draft.
- Product teams can prepare multilingual text variants for review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Add text strings to `items` and set their source and target languages.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "items": [
    {
      "text": "Good morning",
      "source": "en",
      "target": "de"
    }
  ],
  "text": "Good morning"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `items` | array of object | No | Translate multiple text items in one Apify run. Each item must include text, source, and target. Constraints: minimum 1 items; maximum 100 items. |
| `text` | string | No | Single text to translate when items is not provided. |
| `source` | string | No | Single-item source language code. |
| `target` | string | No | Single-item target language code. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "text": "Buenos días, ¿dónde está la estación más cercana?",
  "translated_text": "Good morning, where is the nearest station?",
  "source": "Google Search",
  "target": "es",
  "status_code": 7.3
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-translate-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which Google Translate value should I submit?

Provide the items value listed in the Input table, using the format shown there.

## Related Scrappa Actors

- [YouTube Transcript Scraper](https://apify.com/thescrappa/youtube-transcript-scraper)
- [YouTube Search Scraper](https://apify.com/thescrappa/youtube-api-search-data)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
