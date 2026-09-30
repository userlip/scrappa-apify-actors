# Spec-driven Actors

The 17 pilot Actors in this set are generated from one Rust runtime in `templates/spec-driven-actor/` and one JSON file per Actor in `specs/`. The generated directory is self-contained because Apify uploads only that Actor's directory as `SOURCE_FILES`. Keep endpoint behavior in the spec and shared runtime behavior in the template. Do not edit generated files by hand.

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
   ```

5. Add `actors/<slug>` to the `actor` matrix in `.github/workflows/actor-tests.yml`. Commit the spec, generated directory, and matrix change together. CI also checks that generated directories match their specs and template.

To regenerate every spec-driven Actor after a runtime or README-template change, run:

```bash
node scripts/generate-actors.mjs
node scripts/generate-actors.mjs --check
```

The generator replaces only directories named by files under `specs/`. Review `git status` after generation to confirm it changed only the expected generated Actors.

## Spec fields

| Field | Purpose |
| --- | --- |
| `slug` | Actor directory, package, binary, and Apify Store slug. The spec filename must be `<slug>.json`. |
| `title`, `description` | Actor metadata shown in Apify. |
| `endpoint` | Scrappa API path, including `{path_parameter}` placeholders where needed. |
| `openapiPath` | Source path in the OpenAPI document for traceability. |
| `mode` | `list` extracts array rows; `single` saves the whole response object as one dataset item. |
| `batch` | `field` is the batch array input; `valueField` names the field in each entry; `apiParam` is the Scrappa query parameter; optional `pathParam` maps the value into a path placeholder. |
| `parameters` | Input fields and their Scrappa parameter names, locations, required state, and complete JSON Schema metadata. Use `location: "path"` for path values. Parameters may be shared top-level fields or overridden on an individual batch entry. |
| `endpointByInput` | Optional selector map for inputs that choose between endpoint paths. Each selector value maps to a complete endpoint path. |
| `endpointByInputDefault` | Default selector value when `endpointByInput` is used. |
| `resultPointer` | JSON Pointer to the response array for `list` mode. |
| `fallbackResultPointers` | Optional alternate array pointers, checked in order when the primary pointer is not an array. |
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
| `seo` | Actor-specific Store H1, introduction, keywords, use cases, field descriptions, and related Actor slugs. |

Catalog-only metadata is retained when applicable. The runtime compiles each `spec.json` into its Actor with `include_str!`; it does not read a shared file at run time.

## Runtime behavior

Each Actor accepts a batch of up to 100 entries, calls the Scrappa API, and writes one dataset item per result. It includes the originating input field and `scraped_at`, obeys the run-wide `maxResults` and Apify paid-item limits, and does not write per-item key-value records. The default dataset-item event is charged implicitly by Apify when an item is pushed.

Transient HTTP 429 and 5xx responses are retried with backoff. A request entry that still fails is logged and skipped while later entries continue. A run fails if every entry fails. An empty response is a successful run with zero dataset items and an explicit status message. API credentials are read from `SCRAPPA_API_KEY` at run time.

For local API verification, the runtime supports `APIFY_LOCAL_MODE=1`, `APIFY_LOCAL_STORAGE_DIR`, and `APIFY_LOCAL_INPUT_PATH`. It writes output using the local Apify CLI layout under `storage/datasets/default/`; use synthetic fixtures for automated offline tests and keep live responses outside the repository.

## Pilot notes

Google Play Reviews uses the verified top-level `/reviews` array and cursor fields from the response shape. The production sample and live probes with multiple established app IDs returned an empty array, even with `all_reviews: true`; therefore live verification currently confirms successful execution but cannot confirm real review rows. Recheck this endpoint before publishing its Actor or treating its Store prefill as QA-ready. The Actor unit fixture contains synthetic rows so its offline pointer, pagination, and enrichment behavior remain testable.
