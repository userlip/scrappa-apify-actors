# Spec-driven Actors

The 16 live-verified pilot Actors in this set are generated from one shared runtime in `templates/spec-driven-actor/` and one JSON file per Actor in `specs/`. The generated directory is self-contained because Apify uploads only that Actor's directory as `SOURCE_FILES`. Keep endpoint behavior in the spec and shared runtime behavior in the template. Do not edit generated files by hand.

## Add an Actor

1. Create `specs/<slug>.json`. Start from the closest existing spec, then replace its endpoint, OpenAPI parameters, input prefill, response fixture, result fields, and Store copy with values for the new endpoint.
2. Check the endpoint schema and response shape against the OpenAPI document and a production response. Use `resultPointer` for a list or `mode: "single"` for a whole-object result. Treat catalog candidate arrays as guesses until checked.
3. Generate the standalone crate and Apify metadata:

   ```bash
   node scripts/generate-actors.mjs --only=<slug>
   ```

4. Run the Actor's locked test suite offline and verify all generated files:

   ```bash
   cd actors/<slug>
   cargo test --locked --offline
   cd ../..
   node scripts/generate-actors.mjs --check
   node scripts/validate-store-copy.mjs
   ```

5. Add `actors/<slug>` to the `actor` matrix in `.github/workflows/actor-tests.yml`. Commit the spec, generated directory, and matrix change together. CI also checks that generated directories match their specs and template.

To regenerate every spec-driven Actor after a runtime or README-template change, run:

```bash
node scripts/generate-actors.mjs
node scripts/generate-actors.mjs --check
```

The generator replaces only directories named by files under `specs/`. Review `git status` after generation to confirm it changed only the expected generated Actors. CI runs both the generator sync check and the Store copy validator.

## Spec fields

| Field | Purpose |
| --- | --- |
| `slug` | Actor directory, package, binary, and Apify Store slug. The spec filename must be `<slug>.json`. |
| `title`, `description`, `categories` | Actor name, Store card description (300 characters maximum), and 1 to 3 Store categories. |
| `endpoint` | Scrappa API path, including `{path_parameter}` placeholders where needed. |
| `openapiPath` | Source path in the OpenAPI document for traceability. |
| `mode` | `list` extracts array rows; `single` saves the whole response object as one dataset item. |
| `batch` | `field` is the batch array input; `valueField` names the field in each entry; `apiParam` is the Scrappa query parameter; optional `pathParam` maps the value into a path placeholder. |
| `parameters` | Input fields and their Scrappa parameter names, locations, required state, and complete JSON Schema metadata. Use `location: "path"` for path values. Parameters may be shared top-level fields or overridden on an individual batch entry. `requiredForEndpoints` marks conditional requirements; `availableForEndpoints` omits a parameter from endpoint variants that prohibit it. |
| `endpointSelectors` | Optional one-element array for a selector that chooses the endpoint. Set its `input`, Store-facing `title` and `description`, optional `default`, and labeled `choices` with their endpoint paths. The runtime rejects multiple selectors until it supports them deterministically. |
| `relativeDateDefaults` | Optional rules that fill omitted date inputs at run time, either relative to today or to another input date. Add `endpoints` when a date applies only to selected endpoint variants. Put user-facing default wording in each rule's `description`. Never put fixed dates in `prefill`. |
| `resultPointer` | JSON Pointer to the response array for `list` mode. |
| `fallbackResultPointers` | Optional alternate array pointers, checked in order when the primary pointer is not an array. |
| `flattenPointers` | Optional map from output field names to nested response JSON Pointers. Use it when users need fields such as profile handles and counts as table columns. |
| `dedupePointer` | Optional JSON Pointer to a stable result ID. Duplicate IDs returned across pages for one batch entry are skipped before dataset writes. |
| `pagination` | Optional paging strategy: `kind` is `page` or `cursor`; `param` and `start` define the request input; `nextPointer` and `nextPointers` identify a returned cursor or page; `hasMorePointer`, `endPointer`, `currentPagePointer`, and `totalPagesPointer` provide stop conditions; `step` advances numeric pages; `maxPages` is the hard per-entry bound. |
| `defaultMaxPages` | Default per-entry page limit exposed as the Actor's `maxPages` input. |
| `enrichment.field` | Dataset property containing the input value that produced a result. Every item also receives `scraped_at`. |
| `maxResults` | `input`, `default`, and `hardLimit` define the run-wide dataset item cap. |
| `priceUsdPerResult` | Expected Scrappa price for one upstream result. Generated Store copy displays the per-1,000-results amount. |
| `timeoutSecs` | Apify run timeout. The generated Actor uses 128 MB by default. |
| `prefill` | Complete Apify input used by Store auto-QA. Choose a valid input that returns rows when the upstream endpoint permits it. |
| `testParams` | Catalog-level smoke-test parameters retained for traceability; include them in `prefill` when they form part of the Store test input. |
| `fixture` | Small synthetic response with the real response shape and representative field names. It is embedded in the crate for offline unit tests. Never use a production sample or personal data here. |
| `tableFields` | Most useful dataset fields and Apify table labels/formats. |
| `storeCopy` | Hand-written `seoTitle` (up to 60 characters), `seoDescription` (140 to 155 characters), README `intro`, 3 to 6 `useCases`, at least 3 `howToUse` steps, 1 to 3 Actor-specific `faqs`, 3 to 6 `relatedActors`, and a typed, source-specific `fields` table. The `fields` table covers every meaningful top-level output field, including enrichment and `scraped_at`. |
| `fixture` | Synthetic response used for offline tests and as the README output example. Keep every value realistic and never copy a production response. |

Catalog-only metadata is retained when applicable. The runtime compiles each `spec.json` into its Actor with `include_str!`; it does not read a shared file at run time.

## Runtime behavior

Each Actor accepts a batch of up to 100 entries, calls the Scrappa API, and writes one dataset item per result. It includes the originating input field and `scraped_at`, obeys the run-wide `maxResults` and Apify paid-item limits, and does not write per-item key-value records. The default dataset-item event is charged implicitly by Apify when an item is pushed.

Scrappa HTTP 429, 502, 503, and 504 responses plus connection errors are retried with capped backoff and `Retry-After` support. Each batch entry has a 100-second request budget and each request is limited to 45 seconds. A request entry that still fails is logged with a sanitized upstream message and skipped while later entries continue. The status message lists failed entry indexes, and the run fails if every entry fails. An empty response is a successful run with zero dataset items and an explicit status message. Dataset writes are split into batches of at most 500 items and 5 MB; they are never retried. Apify charge-limit responses stop the run gracefully. API credentials are read from `SCRAPPA_API_KEY` at run time.

For local API verification, the runtime supports `APIFY_LOCAL_MODE=1`, `APIFY_LOCAL_STORAGE_DIR`, and `APIFY_LOCAL_INPUT_PATH`. It writes output using the local Apify CLI layout under `storage/datasets/default/`; use synthetic fixtures for automated offline tests and keep live responses outside the repository.

## Pilot notes

Google Play Reviews is not included in this batch because the production sample and bounded live checks returned no review rows. Scrappa currently builds its upstream request from product ID, locale, store, season ID, and `all_reviews`; the accepted review filter inputs do not change that request. Add the Actor after Scrappa returns review data for a supported input.
