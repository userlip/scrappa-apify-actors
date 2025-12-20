# Scrappa Apify Actors

Apify Actors for [Scrappa](https://scrappa.co) APIs - Google Search, Maps, YouTube, LinkedIn, and more.

## Structure

```
scrappa-apify-actors/
├── actors/           # Individual Apify actors
│   ├── google-search/
│   ├── google-maps-business/
│   └── ...
├── shared/           # Shared Scrappa client library
└── package.json      # Workspace root
```

## Setup

```bash
# Install pnpm if you don't have it
npm install -g pnpm

# Install dependencies
pnpm install

# Build all packages
pnpm build
```

## Development

### Running an actor locally

```bash
cd actors/google-search
pnpm start:dev
```

### Creating a new actor

1. Copy an existing actor as a template:
```bash
cp -r actors/google-search actors/my-new-actor
```

2. Update `package.json` with the new name
3. Update `.actor/actor.json` with metadata
4. Update `.actor/input_schema.json` with input fields
5. Implement the actor logic in `src/main.ts`

## Deploying to Apify

### First time setup

```bash
# Login to Apify
apify login

# Push an actor
cd actors/google-search
apify push
```

### Updating an actor

```bash
cd actors/google-search
pnpm build
apify push
```

## Available Actors

| Actor | Description | Status |
|-------|-------------|--------|
| `google-search` | Google Search results | ✅ Ready |
| `google-search-light` | Lightweight Google Search | 🔜 Planned |
| `google-maps-business` | Google Maps business info | 🔜 Planned |
| `google-maps-reviews` | Google Maps reviews | 🔜 Planned |
| `google-jobs` | Google Jobs listings | 🔜 Planned |
| `google-news` | Google News results | 🔜 Planned |
| `google-images` | Google Images search | 🔜 Planned |
| `youtube-search` | YouTube video search | 🔜 Planned |
| `youtube-video` | YouTube video details | 🔜 Planned |
| `linkedin-profile` | LinkedIn profile scraping | 🔜 Planned |
| `linkedin-company` | LinkedIn company info | 🔜 Planned |
| `trustpilot-search` | Trustpilot business search | 🔜 Planned |
| `trustpilot-reviews` | Trustpilot reviews | 🔜 Planned |
| `kununu-search` | Kununu company search | 🔜 Planned |
| `kununu-reviews` | Kununu company reviews | 🔜 Planned |
| `brave-search` | Brave Search results | 🔜 Planned |
| `startpage-search` | Startpage Search results | 🔜 Planned |

## API Key

All actors require a Scrappa API key. Get yours at [https://scrappa.co](https://scrappa.co).
