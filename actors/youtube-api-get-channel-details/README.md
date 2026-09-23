# YouTube API Get Channel Details

Fetch YouTube channel details from Scrappa's YouTube API and save the result to the Apify default dataset.

Set `SCRAPPA_API_KEY` as an Actor secret before running. The actor accepts the batch `ids` field and the legacy single-channel `id` field; if both are provided, unique IDs are processed in order with `ids` first.

```json
{
    "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

Scrappa's response rows are written to the default dataset in response order, up to the run's remaining pay-per-event spending budget; an array response is trimmed to its first affordable rows, and later rows (including error rows) are skipped once the budget is exhausted. A failed channel produces one error row with its ID and error message when budget remains; the actor continues through the batch and fails the run if every channel fails or an Apify pricing/storage operation fails.

## Local Development

Build and run with the Rust toolchain. The actor reads input from the Apify default key-value store and writes results to the default dataset using the `ACTOR_DEFAULT_KEY_VALUE_STORE_ID`, `ACTOR_DEFAULT_DATASET_ID`, `ACTOR_RUN_ID`, `ACTOR_INPUT_KEY`, and `APIFY_TOKEN` runtime variables. `APIFY_API_PUBLIC_BASE_URL` and `SCRAPPA_API_BASE_URL` can be overridden for local API mocks.

```bash
cargo run --locked
```
