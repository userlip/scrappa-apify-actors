import { Actor } from 'apify';
import { parseInput } from './input.js';
import type { ChallengePostsInput } from './input.js';
import { scrapeChallenge } from './scrape.js';
import { ScrappaClient } from './shared/scrappa-client.js';

await Actor.init();
try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set');
    const input = await Actor.getInput<ChallengePostsInput>();
    if (!input) throw new Error('Actor input is required');

    const requests = parseInput(input);
    const client = new ScrappaClient(apiKey);
    const summaries = [];
    for (const request of requests) {
        const summary = await scrapeChallenge(client, Actor, request);
        summaries.push(summary);
        console.log(JSON.stringify(summary));
        if (summary.status === 'charge-limit-reached') break;
    }

    const saved = summaries.reduce((total, item) => total + item.videos_saved, 0);
    const failed = summaries.filter((item) => item.status === 'failed').length;
    console.log(`Completed ${summaries.length}/${requests.length} challenges: ${saved} videos saved, ${failed} partial failures.`);
} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Actor failed: ${message}`);
    await Actor.fail(message);
}
await Actor.exit();
