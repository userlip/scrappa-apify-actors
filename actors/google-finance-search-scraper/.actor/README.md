# Google Finance Search Scraper

This Rust actor searches Google Finance through Scrappa's `/api/google-finance/search` endpoint. Use `q` for one search or `queries` for up to 25 searches per run. It stores one default-dataset item per matched finance result and charges the `finance-search-result` event for each saved result on pay-per-event runs.
