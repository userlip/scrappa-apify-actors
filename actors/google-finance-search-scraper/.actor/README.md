# Google Finance Search Scraper

Search Google Finance for stocks, ETFs, indices, funds, currencies, and other finance instruments by ticker or company name. The actor is a thin Scrappa wrapper, supports batch `queries`, and writes one dataset item per matched finance result.

Use `q` for one search, or `queries` for up to 25 searches in one Apify run. Heavy scraping is handled by Scrappa; Apify only validates input, calls `https://scrappa.co/api/google-finance/search`, and stores dataset results.
