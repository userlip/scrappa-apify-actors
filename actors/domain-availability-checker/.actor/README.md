# Domain Availability Checker

Check domain registration status and registry events using public RDAP records. Submit one domain or a batch of names to check their current RDAP registration status.

## What data can you extract?

Registration status and dates reflect the registry data returned through RDAP.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means RDAP provided no flag. |
| `domain` | text | Domain shown for the domain lookup by RDAP, in the format used by the source; null when it is omitted. |
| `available` | boolean | True when RDAP indicates the domain can be registered; false when it has an active registration, null if status is unavailable. |
| `registered` | boolean | Whether the domain has a registry record; false is a reported value, while null means RDAP provided no flag. |
| `status` | text | Status reported for the domain lookup by RDAP; null when RDAP does not provide the value. |
| `confidence` | text | confidence for the domain lookup, assigned by RDAP; null when the source does not expose it. |
| `source` | text | Source or language label shown for the domain lookup by RDAP, in the format used by the source; null when it is omitted. |
| `rdap_url` | link | Rdap url for this domain lookup on RDAP; null when the source does not provide a URL. |
| `rdap_status_code` | number | Rdap status code shown for the domain lookup by RDAP, in the format used by the source; null when it is omitted. |
| `rdap_events` | array | Domain registry events with event action, date and registrar or registry from RDAP; an empty list when no entries are available. |
| `nameservers` | array | DNS nameserver hostnames listed in the RDAP record; an empty list when no entries are available. |
| `message` | text | Diagnostic text for the RDAP lookup; null when the request completes without an error. |
| `error` | text | Diagnostic text for the RDAP lookup; null when the request completes without an error. |
| `status_code` | number | Http status code shown for the domain lookup by RDAP, in the format used by the source; null when it is omitted. |
| `input_domain` | text | Domain name passed to RDAP. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Brand teams can check candidate domains before shortlisting names.
- Web operations teams can compare RDAP status and registration events.
- Product teams can verify domain status during naming or migration work.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `domains` and use the identifier or URL format required by the RDAP registry directory.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "domains": [
    "example.com"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domains` | array of string | No | Domain names or URLs to check in one Apify run. Batch this field to reduce run overhead and get one dataset item per checked domain. Constraints: minimum 1 items. |
| `domain` | string | No | Backward-compatible single domain input. Prefer Domains for bulk checks. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "rdap_events": [
    {
      "action": "registration",
      "date": "2024-06-12T18:24:00Z",
      "actor": "Example Registrar"
    }
  ],
  "domain": "northstar.example",
  "available": true,
  "registered": false,
  "status": "active",
  "confidence": "high",
  "source": "Google Search",
  "rdap_url": "https://rdap.example.net/domain/northstar.example"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~domain-availability-checker/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What does an RDAP result tell me about a domain?

Registration status and event details reflect the RDAP response. Missing event or confidence values mean the registry did not provide them.

## Related Scrappa Actors

- [Website Content Extractor](https://apify.com/thescrappa/website-content-extractor-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
