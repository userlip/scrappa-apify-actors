# Testing Report: Google Maps Directions Scraper

Date: 2026-07-13

## Verification refresh

Fresh local verification completed at 2026-07-13T14:35:46Z from
`actors/google-maps-directions-scraper`. No application source changes were
made during this testing stage.

| Check | Result |
| --- | --- |
| `npm test` | 16 passed, 0 failed |
| `npm run typecheck` | Passed |
| `jq empty .actor/actor.json .actor/input_schema.json` | Passed |
| `npx apify-cli validate-schema` | Input and dataset schemas passed |
| `npm audit --omit=dev --audit-level=high` | 0 vulnerabilities |
| `git diff --check origin/main...HEAD` | Passed |

## Result

TESTING_PASSED

Testing passed. Local verification, deployment configuration, paid pricing, a two-route live smoke run, and a mixed-success/no-charge smoke run all passed after restoring the missing deployed secret.

## Local verification

All commands were run from `actors/google-maps-directions-scraper`:

| Check | Result |
| --- | --- |
| `npm test` | 16 passed, 0 failed |
| `npm run typecheck` | Passed |
| `jq empty .actor/actor.json .actor/input_schema.json` | Passed |
| `npx apify-cli validate-schema` | Input and dataset schemas passed |
| `npm audit --omit=dev --audit-level=high` | 0 vulnerabilities |

The focused tests include the synthetic `apify-default-dataset-item` charge regression: a PPE writer response with `chargedCount: 2` is normalized to one `route-result` charge for one stored route row.

## Deployment and release gates

Actor: `ZF8jFdzF15k49AZQh` (`google-maps-directions-scraper`)

- Public: `true`.
- Active `PAY_PER_EVENT` pricing is API-verified through `pricingInfos` at `$0.0005` for `route-result` (`startedAt: 2026-07-13T12:10:04.000Z`).
- Actor default run profile is API-verified at 128 MB memory and 300 seconds timeout.
- Version `1.0` has `SCRAPPA_API_KEY` configured as a secret; the API returned a secret value hash, not the value.
- Build `1.0.5`, ID `WyXaXBr5ncpnhxIpl`, finished `SUCCEEDED` at `2026-07-13T13:19:45.013Z`.

The first post-review smoke run (`NUlezwj5rcaC7b7ck`) failed before network work because the deployed runtime lacked `SCRAPPA_API_KEY`; it produced zero dataset rows and zero charges. The secret was restored from the configured local secret store, version `1.0` was rebuilt, and all subsequent live checks used build `1.0.5`.

## Live two-route smoke verification

Run: `X3G3VmrqE91coTP8B`

Dataset: `H7S7YVb70euFiVuZS`

- Input contained two distinct requests in one run: Berlin Hauptbahnhof → Brandenburg Gate walking and driving.
- Terminal status: `SUCCEEDED`.
- Six dataset rows were stored: three walking alternatives and three driving alternatives.
- Final API `chargedEventCounts.route-result`: 6.
- Dataset rows and `route-result` charges matched: 6 = 6.
- Log summary reported `requested: 2`, `succeeded: 2`, `failed: 0`, `alternativesSaved: 6`, and `charged: 6`.

## Mixed-failure and charge-parity verification

Run: `NwhoCso1d0soax99d`

Dataset: `52vk0XK01jUMqvTHJ`

- Input contained one valid Berlin walking request and one deliberately invalid place pair.
- Terminal status: `SUCCEEDED`.
- The valid request produced three dataset rows.
- The invalid request produced one logged failure with `NO_ROUTES_FOUND`, no dataset rows, and no charge.
- Final API `chargedEventCounts.route-result`: 3.
- Dataset rows and `route-result` charges matched: 3 = 3.
- Log summary reported `requested: 2`, `succeeded: 1`, `failed: 1`, `alternativesSaved: 3`, and `charged: 3`.

No per-item key-value-store writes were observed; the successful smoke run recorded one key-value-store write for the run-level output path and six dataset writes for six monetized rows.

## Reproduction commands

Local:

```text
cd actors/google-maps-directions-scraper
npm test
npm run typecheck
jq empty .actor/actor.json .actor/input_schema.json
npx apify-cli validate-schema
npm audit --omit=dev --audit-level=high
```

Live runs were started through the Apify API for actor `ZF8jFdzF15k49AZQh` with the two-route and mixed-failure JSON inputs described above. Run and dataset IDs are recorded in this report for API/log inspection.
