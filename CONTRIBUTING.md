# Contributing

## README actor inventory

When you deploy a new actor to Apify, or rename or retire an existing live actor, update the inventory table in `README.md` in the same change.

Keep each inventory row in sync with the live `TheScrappa` Apify org:
- actor slug
- actor ID
- actor title
- source coverage status

If an actor is live in Apify but its source directory is not present in this repository, keep it listed in `README.md` and mark it as missing local source coverage instead of omitting it.
