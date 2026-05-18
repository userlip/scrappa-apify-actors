# YouTube API Get Channel Details

Fetch YouTube channel profile details from Scrappa's YouTube API and save the result to the Apify default dataset.

## Input

Provide a YouTube channel ID.

```json
{
    "id": "UCJZv4d5rbIKd4QHMPkcABCw"
}
```

## Output

The actor saves the channel details returned by Scrappa to the default dataset. A typical result includes fields such as:

```json
{
    "id": "UCJZv4d5rbIKd4QHMPkcABCw",
    "name": "Kevin Powell",
    "description": "Helping you learn how to make the web...",
    "thumbnail": "https://yt3.googleusercontent.com/...",
    "banner": "https://yt3.googleusercontent.com/...",
    "subscriberCount": "1M subscribers 1.1K videos",
    "videoCount": "Unavailable",
    "viewCount": "Unavailable",
    "country": "Unavailable",
    "joinedDate": "Unavailable",
    "verified": false
}
```

The API may include additional channel metadata depending on YouTube availability.

## Local Development

Install dependencies and run the actor with a local Apify input file:

```bash
npm install
mkdir -p storage/key_value_stores/default
printf '{"id":"UCJZv4d5rbIKd4QHMPkcABCw"}' > storage/key_value_stores/default/INPUT.json
npm start
```

Run the lightweight syntax check:

```bash
npm test
```
