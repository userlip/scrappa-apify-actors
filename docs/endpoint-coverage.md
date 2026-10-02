# Scrappa endpoint coverage on Apify

Checked 2026-09-30 against `https://scrappa.co/api/docs/openapi.json` (318 paths).

- 106 live Actors already cover the main data endpoints (see README inventory).
- `docs/endpoint-actor-catalog.json` lists the 87 data endpoints that were candidates for a new Actor. Each entry was probed live against production Scrappa with the listed `test_params`.
- Result (2026-10-02): 81 new Actors were generated, live-verified on Apify and queued for publication (see the README table). Together with the 106 existing Actors, Scrappa now has 187 Apify Actors.

## Catalog endpoints that did not ship

| Endpoint | Evidence | Next step |
|---|---|---|
| `/google/play/product/reviews` | Returned an empty `reviews` array for every tested app and parameter combination, including `all_reviews` | Fix review extraction in Scrappa, then add the spec |
| `/immowelt/property` | Live probes did not return usable listing data | Re-test after the Immowelt endpoint is fixed |
| `/vinted/similar-items` | Live probes did not return usable rows | Re-test after the Vinted endpoint is fixed |
| `/tiktok/user/favorites` | HTTP 200 with zero videos for public accounts | Only ship if a reliable public example exists |
| `/tiktok/user/story` | Stories expire, so no prefill can return rows every day for Apify QA | Not suitable for a Store Actor |
| `/instagram/audio/reels` | No reliably discoverable public audio IDs; probes returned `found: false` | Re-test with a known audio ID |

## Known upstream reliability issues

- `/google/lens` is blocked by Google on roughly half of the calls (HTTP 503 `GOOGLE_LENS_BLOCKED`); the generated Actor retries for about a minute.
- The private Actors `youtube-api-get-channel-shorts`, `youtube-api-get-channel-livestreams`, `youtube-api-channel-podcasts` and `youtube-api-playlists` return no rows because the YouTube backend does not currently return those video types; they stay private.

## Endpoints intentionally without their own Actor

| Endpoint group | Reason |
|---|---|
| `/account/usage`, `/docs*`, `/zapier/oauth/token` | Account and infrastructure, not data |
| Reference lists: `/flights/airlines`, `/flights/airports`, `/kayak/reference/*`, `/trustpilot/categories`, `/trustpilot/countries`, `/trustedshops/markets`, `/trustedshops/categories`, `/vinted/countries`, `/vinted/categories`, `/vinted/filters`, `/tiktok/regions`, `/youtube/locales`, `/kununu/industries`, `/semrush/facets`, `/kleinanzeigen/shipping-options` | Static lookup tables. They are used as input options inside the Actors instead |
| Location and ID resolvers: `/arbeitsagentur/locations`, `/jameda/autocomplete-*`, `/kununu/autocomplete`, `/kununu/job-locations`, `/kununu/job-titles`, `/kununu/closest-cities`, `/stepstone/autosuggest`, `/redfin/locations`, `/immowelt/locations`, `/immowelt/places/*`, `/immowelt/count`, `/realestate-com-au/locations`, `/realestate-com-au/autocomplete`, `/ohne-makler/locations`, `/idealista/locations`, `/idealista/location`, `/booking/destinations`, `/kayak/flights/airports`, `/economy-car-rentals/autocomplete`, `/rentalia/locations` | Helpers that turn a text into an internal ID. Actors accept plain text or URLs where possible |
| Typed product variants: `/apple/app-store/{apps,books,audiobooks,movies,music,podcasts,tv}/product`, `/apple/app-store/product`, `/google/play/{apps,books,audiobooks,movies,tv}/product`, `/google/play/product` | Covered by the App Details Actors through a `store` input |
| Idealista internals: `/idealista/deeplink/*`, `/idealista/shorturis`, `/idealista/map/*`, `/idealista/filters`, `/idealista/home`, `/idealista/news`, `/idealista/phones/prefixes`, `/idealista/listing/multimedia`, `/idealista/listing/stats`, `/idealista/vacation/calendar` | Map tiles, URL helpers and sub-resources of a listing. The listing Actor returns the full listing |
| `/booking/bulk-prices`, `/booking/search-by-url` | Covered by Booking Prices and Booking Search |
| `/similarweb/batches*` | Covered by the SimilarWeb Actor, which batches inside one run |
| `/semrush/content/*`, `/semrush/local/review-qr` | AI text generation and QR helpers, not data extraction |
| `/semrush/sensor/ranks`, `/semrush/sensor/serp-features` | Covered by the Semrush Sensor Actor |
| `/rentalia/search`, `/rentalia/geo-search`, `/rentalia/house`, `/rentalia/price` | Rentalia discontinued its search upstream (HTTP 410) |
| `/tiktok/user/similar` | Upstream deprecated (HTTP 503 `tiktok_similar_users_upstream_deprecated`) |
| `/instagram/audio/reels`, `/instagram/search/popular`, `/instagram/user/basic`, `/instagram/user/embed`, `/v2/instagram/user` | Unreliable or login-gated upstream, or duplicate of the Instagram User Info Actor |
| `/kununu/review`, `/kununu/similar-jobs`, `/kununu/profiles`, `/kununu/sitemap`, `/kununu/recommended-local`, `/kununu/related-job-searches`, `/kununu/salaries`, `/kununu/top-company-jobs` | Sub-resources or directory pages covered by the Kununu Actors |
| `/maps/review`, `/search-advanced*`, `/search-light` | Covered by existing Google Maps Reviews and Google Search Actors |
| `/mobile-de/v1/dealer-rating`, `/mobile-de/v1/recommendations`, `/tiktok/collections/{list,details}`, `/tiktok/playlists/{list,details}`, `/vinted/item-shipping`, `/vinted/suggestions`, `/billiger/{product,baseproduct,shop}`, `/realestate-com-au/{agent-profile,agency-profile,sales-events}`, `/flights/booking-details`, `/kayak/flights/round-trip` (a trip-type option of Kayak Flights), `/kayak/stays/media`, `/kayak/cars/insights`, `/kayak/direct/routes`, `/kayak/flights/route-intelligence`, `/kayak/guides/city`, `/baidu/developer-search`, `/baidu/translation-suggestions`, `/web-scraper` (Website Content Extractor) | Sub-resources, options of another Actor, or low-demand variants. Candidates for a later wave |
