# Apify Store copy style guide (Scrappa)

This guide is the quality bar for every Scrappa Actor listing: title, description, SEO title, SEO description, categories and README. The listing is the product page. It must read like a human product writer who knows the data, not like a template.

## Hard rules

- English. No em dashes. No emojis.
- Never mention internal things: endpoint paths (`/kayak/flights/one-way`), "Scrappa API endpoint", Rust, ports, QA runs, run IDs, build numbers, fixtures, event names like `challenge-post-result`.
- No field names in backticks inside title, description or SEO fields. Field names belong in the README tables only.
- No filler phrases. Banned (validator enforces): "returned for this result", "Provide the fields listed below", "Results can include", "Save and export Apify datasets", "Search terms", "collects structured data from", "for Campaign Research", "for Creator Research", "for Audience Research", "for Lead Research", "for Market Research", "for Video Analysis", "for Hiring Teams", "Each entry maps its".
- Never promise data, filters or limits the code does not deliver. If a field is sometimes empty, say when.
- Output examples must look real: realistic synthetic values for every field shown, never `{}` or `[]` placeholders, never `"Example result"`, never `42` repeated. Anonymize private people (invented names), public brands and places are fine.

## Title (Store name)

- Pattern: `<Platform> <What> Scraper` or a clear product name. Natural length, usually 20 to 45 characters. Do not pad to hit a length.
- Use the words people type into Google and the Apify search: "Google Maps Reviews Scraper", "Kayak Flights Scraper", "Idealista Scraper".
- Keep these existing base titles unchanged (they rank and convert): Google Maps Advanced Search Scraper, LinkedIn Company Scraper, LinkedIn Profile Scraper, Google Search Scraper (`google-search-scraper`), Google Images Scraper, Instagram User Info | Cheapest $0.20/1k results, Instagram Post Info | Cheapest $0.20/1k results, Google Maps Photos Scraper, Vinted Search Scraper, Trustpilot Company Reviews Scraper.
- LinkedIn Company, LinkedIn Profile and LinkedIn Search Scraper may keep the live ` - $0.30/1k results` suffix only while each Actor's latest price remains $0.30 per 1,000 results. Use this dash format consistently.
- Disambiguate siblings by what you get, not by audience: "TikTok Hashtag Videos Scraper" vs "TikTok Hashtag Details Scraper".

## Description (max 300 characters, shown on the Store card and page header)

- Sentence 1: what you get, from where, in plain words, with the 3 to 5 most valuable data points.
- Sentence 2: how you use it (what you paste in, batch support) and one real differentiator (no login, no proxies needed, fast, cheap per 1,000, pagination, cached results), only if true for this Actor.
- Vary the wording across Actors. Do not end every description with the same sentence.

Gold examples:

- Google Maps Reviews Scraper: "Export Google Maps reviews with star rating, full text, date, owner reply and reviewer name for any place. Paste place IDs or Maps URLs, scrape many places in one run and sort by newest or most relevant. No Google login needed."
- Kayak Flights Scraper: "Get Kayak flight offers for one-way and round trips with price, airlines, stops, duration, departure and arrival times and booking providers. Add several routes in one run to compare fares or track prices over time."
- Idealista Scraper: "Scrape Idealista property listings in Spain, Italy and Portugal with price, size, rooms, floor, photos, location and agency. Search by location and filters, page through results and export everything to JSON, CSV or Excel."
- Semrush Domain Overview Scraper: "Get a Semrush-style domain overview for any website: estimated visits, traffic trend, top countries, traffic sources and category rank. Check hundreds of domains in one run for competitor research or lead scoring."
- Impressum Contact Data Extractor: "Extract company name, address, email, phone, managing directors and VAT ID from the Impressum (legal notice) of German, Austrian and Swiss websites. Paste a list of domains and get one clean contact record per company."

## SEO title (max 60 characters, Google result title)

- Lead with the main keyword, then a benefit or alternative keyword after ` | ` or ` - `... use ` | `.
- Examples: "Google Maps Reviews Scraper | Export Reviews to CSV", "Kayak Flights Scraper | Flight Prices API", "Idealista Scraper | Spain, Italy and Portugal Listings", "Impressum Scraper | German Company Contact Data".

## SEO description (140 to 155 characters, Google snippet)

- Answer the search intent in the first 80 characters. Include the main keyword once and one secondary keyword ("API", "export to CSV", "without login", "prices", "reviews").
- Example: "Scrape Kayak flight prices for any route. Get airlines, stops, times and booking providers for one-way and round trips. Export to CSV or use the API."

## Categories (1 to 3, most specific first)

- Real estate portals: REAL_ESTATE first. Job boards and employer data: JOBS first. YouTube, TikTok video data: VIDEOS or SOCIAL_MEDIA first. Travel: TRAVEL. Shops and price comparison: ECOMMERCE. Search engines and SEO tools: SEO_TOOLS. Company contact and B2B data: LEAD_GENERATION. Reviews of companies: MARKETING or BUSINESS. Finance: BUSINESS.
- Use DEVELOPER_TOOLS only as a third category, never first. Do not use EDUCATION or FOR_CREATORS unless the Actor clearly targets them.

## README structure

1. `# <Title>` then a 2 to 3 sentence intro in the same spirit as the description (what, from where, why it is useful).
2. `## What data can you extract?` table: field, type, and a description that says what the value means for this source, with units and formats ("Price in the listing currency, as a number", "ISO 8601 date the review was posted", "Star rating from 1 to 5"). Every row must be specific. Group nested objects ("`author`: object with `name`, `username`, `followers`").
3. `## Use cases` 3 to 6 concrete bullets naming who uses it and for what outcome.
4. `## How to use` numbered steps plus a working input JSON with batch input where supported. Dates in examples must not be hard-coded past dates; prefer inputs that work any day.
5. `## Input` table generated from the input schema.
6. `## Output example` one realistic record (see rules above).
7. `## Pricing` the live price, e.g. "$0.30 per 1,000 results", plus one sentence on what counts as a result.
8. `## FAQ` legality of public data, limits, API usage with the real `POST https://api.apify.com/v2/acts/thescrappa~<name>/runs` URL, integrations (Make, Zapier, n8n, Google Sheets), what happens on errors, plus 1 to 3 Actor-specific questions people actually ask (e.g. "How do I find the Idealista location ID?").
9. `## Related Scrappa Actors` 3 to 6 links to real sibling Actors.
