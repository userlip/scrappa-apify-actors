# Similarweb Traffic Analytics Scraper

Analyze website traffic and competitive metrics for one or more domains. The actor writes one dataset item per analyzed domain and uses Scrappa's `/api/similarweb` endpoint for the actual data collection.

Use `domains` to batch many websites into one Apify run. URLs, `www` prefixes, uppercase domains, and paths are normalized before the Scrappa API call.
