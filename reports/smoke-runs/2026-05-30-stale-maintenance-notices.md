# 2026-05-30 Stale Maintenance Notice Clearance

Scope: verify stale Apify `UNDER_MAINTENANCE` notices on public monetized TheScrappa actors and clear only notices backed by successful cloud run evidence.

| Actor | Actor ID | Evidence run | Dataset items | Log evidence | Final notice state |
| --- | --- | --- | ---: | --- | --- |
| `booking-search-scraper` | `BehWN3LEvBxhEiJDF` | `fGubXwuYYNmoMyakt` | 28 | Run succeeded on build `NR534OiXB7Rb8wB3r`; log reported `Booking.com search completed successfully` and no error-like lines. | Cleared; Apify detail returned `notice: null`. |
| `google-videos-scraper` | `kAdTwn5fkBCGKOQUq` | `oGeUylDZScdeKwCF1` | 10 | Run succeeded on build `k28NhmnFcnbL6pHRH`; log reported `Google Videos scraping completed successfully` and no error-like lines. | Cleared; Apify detail returned `notice: null`. |
| `google-hotels-search-scraper` | `Kc3rfsV2Hif23mctw` | `0kc6kUen9F8Z53MAK` | 20 | Run succeeded on build `lbbXlKllL85dphblP`; log reported `Google Hotels search completed successfully` and no error-like lines. | Cleared; Apify detail returned `notice: null`. |
| `youtube-transcript-scraper` | `ztc698cHC09lkCDYE` | `jwH7cRhoY6OMZp9l8` | 1 | Run succeeded on build `QCcV5FQUckuBydA4f`; log reported `Successfully fetched transcript with 61 segment(s)` and no error-like lines. | Cleared; Apify detail returned `notice: null`. |

Notes:

- Booking was not re-run because run `fGubXwuYYNmoMyakt` was recent successful release evidence with 28 dataset items.
- Google Videos smoke input used query `coffee brewing tutorial`, `page: 1`, `hl: en`, `gl: us`, `google_domain: google.com`, `safe: off`.
- Google Hotels smoke input used `Paris, France` with 2026-07-01 to 2026-07-03 stay dates, `adults: 2`, `currency: EUR`, `gl: fr`, `hl: en`.
- YouTube Transcript smoke input used video ID `dQw4w9WgXcQ`, `language: en`, `hl: en`, `gl: US`.
- Google Hotels emitted Apify warning `Attempting to charge for an unknown event 'hotel-result'`; local source was updated to stop passing the obsolete explicit charge event and rely on the configured default dataset item pricing.
