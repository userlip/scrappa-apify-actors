# Testing Handoff: Google Maps Directions Scraper

Testing passed for `google-maps-directions-scraper`.

- Local `npm test`: 16 passing; TypeScript, schema validation, and high-severity dependency audit passed.
- Deployed Actor `ZF8jFdzF15k49AZQh` is public, uses 128 MB/300 seconds, retains `SCRAPPA_API_KEY` as a secret, and has active `$0.0005` `route-result` pricing.
- Build `1.0.5` (`WyXaXBr5ncpnhxIpl`) succeeded.
- Two-route smoke `X3G3VmrqE91coTP8B` stored 6 rows and persisted 6 `route-result` charges; the log summary reported `charged: 6`.
- Mixed-failure smoke `NwhoCso1d0soax99d` stored and charged 3 valid rows, continued after one `NO_ROUTES_FOUND` failure, and charged nothing for the failed request.

The earlier missing-secret run (`NUlezwj5rcaC7b7ck`) was remediated before the passing verification runs.
