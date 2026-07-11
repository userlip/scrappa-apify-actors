import { Actor } from 'apify';
import { parseInput } from './input.js';
import type { ChallengePostsInput } from './input.js';
import { scrapeChallenge } from './scrape.js';
import { isTotalFailure } from './run-summary.js';
import { ScrappaClient } from './shared/scrappa-client.js';

async function main(): Promise<void> {
    await Actor.init();
    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set');
        const input = await Actor.getInput<ChallengePostsInput>();
        if (!input) throw new Error('Actor input is required');

        const requests = parseInput(input);
        const client = new ScrappaClient(apiKey);
        const summaries = [];
        const seenVideoIds = new Set<string>();
        for (const request of requests) {
            const summary = await scrapeChallenge(client, Actor, request, seenVideoIds);
            summaries.push(summary);
            console.log(JSON.stringify(summary));
            if (summary.status === 'charge-limit-reached') break;
        }

        const saved = summaries.reduce((total, item) => total + item.videos_saved, 0);
        const incomplete = summaries.filter((item) => item.status !== 'succeeded').length;
        console.log(`Completed ${summaries.length}/${requests.length} challenges: ${saved} videos saved, ${incomplete} incomplete challenges.`);
        if (isTotalFailure(summaries)) {
            await Actor.fail('Every challenge failed before producing a video. Check the challenge IDs, Scrappa API key, and upstream availability.');
            return;
        }
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }
    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Actor failed: ${message}`);
    process.exitCode = 1;
});
