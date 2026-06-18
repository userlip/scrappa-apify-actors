# LinkedIn Job Details Scraper

Apify actor wrapper for Scrappa's LinkedIn Job endpoint. The actor accepts one or more LinkedIn job posting URLs, calls `/linkedin/job` once per normalized URL, and saves one dataset item per URL.

Use the batch `urls` input for normal usage so one Apify run can process many job URLs.
