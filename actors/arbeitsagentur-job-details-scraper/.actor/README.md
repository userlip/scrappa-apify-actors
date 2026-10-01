# Arbeitsagentur Job Details Scraper

Retrieve public German Federal Employment Agency vacancy details by job reference number. Collect role descriptions, employer details, pay information, contract terms, and work locations.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the requested vacancy record was retrieved successfully. |
| `data` | Object | Arbeitsagentur vacancy details including role, employer, pay, contract, publication, and location data. |
| `input_refnr` | String | Arbeitsagentur job reference number used for this vacancy lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Recruiters can review public vacancy requirements and employment conditions.
- Labor-market analysts can compare salary and contract details across job listings.
- Job platforms can enrich vacancy records with structured employer and location data.

## How to use

1. Add one Arbeitsagentur reference number to each entry in `job_refs`.
2. Set the result cap for the number of vacancy records to save.
3. Export the structured job details for your recruiting or labor-market workflow.

```json
{
  "job_refs": [
    {
      "refnr": "12016-10005410012-S"
    }
  ],
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `job_refs` | Array\<object\> | Yes | Arbeitsagentur reference numbers for individual public job listings. |
| `job_refs[].refnr` | string | Yes per entry | Arbeitsagentur reference number that identifies a public vacancy. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "stellenangebotsTitel": "Senior Data Analyst (m/w/d)",
    "stellenangebotsBeschreibung": "Analyze operational data, build clear reporting, and collaborate with product and engineering teams.",
    "arbeitszeitVollzeit": true,
    "eintrittszeitraum": "Nach Vereinbarung",
    "verguetungsangabe": "58.000 bis 72.000 EUR jährlich",
    "artDerVerguetung": "Jahresgehalt",
    "festgehalt": true,
    "vertragsdauer": "Unbefristet",
    "datumErsteVeroeffentlichung": "2026-09-12",
    "aenderungsdatum": "2026-09-25",
    "firma": "Nordstern Energie GmbH",
    "referenznummer": "12016-10005410012-S",
    "stellenlokationen": [
      {
        "ort": "Berlin",
        "land": "Deutschland",
        "plz": "10115"
      }
    ]
  },
  "input_refnr": "12016-10005410012-S",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~arbeitsagentur-job-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where can I get a reference number?

Use Arbeitsagentur Jobs Scraper to find active listings and copy each result reference number into this Actor.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Kununu Job Details Scraper](https://apify.com/thescrappa/kununu-job-details-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
