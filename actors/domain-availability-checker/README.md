# Domain Availability Checker

Check domain availability in bulk using Scrappa's RDAP-powered Domain Availability API.

This actor is designed for brand research, SEO project planning, domain investor lists, and quick availability pre-checks. It accepts many domains in one Apify run and writes one dataset item per domain.

## Input

Use `domains` for batches:

```json
{
  "domains": [
    "example.com",
    "https://scrappa.co/pricing",
    "brand-name-test-availability-2026.com"
  ]
}
```

`domain` is also supported for single-domain compatibility, but batching with `domains` is recommended.

## Output

Each dataset item includes:

- `domain`
- `available`
- `registered`
- `status`
- `confidence`
- `source`
- `rdap_url`
- `rdap_status_code`
- `rdap_events`
- `nameservers`
- `message`
- `error` for per-domain failures

## RDAP availability caveat

RDAP can show that a domain is registered and can indicate that a domain is probably available when the registry returns not found. It does not prove registrar checkout availability, premium pricing, aftermarket inventory, reserved names, or registration restrictions.

For high-volume usage or direct API access, use Scrappa at `https://scrappa.co/api`.
