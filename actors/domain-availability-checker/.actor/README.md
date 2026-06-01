# Domain Availability Checker

Bulk-check domains with Scrappa's RDAP-based availability endpoint.

## Why use it

- Check brand and product name candidates
- Screen SEO project domain lists
- Process many domains in a single Apify run
- Get one dataset row per checked domain

## Example input

```json
{
  "domains": [
    "example.com",
    "https://scrappa.co",
    "brand-name-test-availability-2026.com"
  ]
}
```

## Example output

```json
{
  "success": true,
  "input_domain": "example.com",
  "domain": "example.com",
  "available": false,
  "registered": true,
  "status": "registered",
  "confidence": "high",
  "source": "rdap",
  "rdap_url": "https://rdap.verisign.com/com/v1/domain/example.com",
  "rdap_status_code": 200,
  "nameservers": ["A.IANA-SERVERS.NET", "B.IANA-SERVERS.NET"]
}
```

RDAP availability is a strong pre-check, not a registrar checkout guarantee. Domains marked `probably_available` can still be premium, reserved, restricted, or unavailable at a specific registrar.
