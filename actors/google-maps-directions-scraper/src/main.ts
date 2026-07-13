import { Actor } from 'apify';
import { runDirectionsBatch } from './batch-runner.js';
import { createChargedRouteWriter } from './charged-save.js';
import { buildDirectionsRequests } from './request-params.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import type { DirectionsInput } from './request-params.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 45000;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<DirectionsInput>();
        const requests = buildDirectionsRequests(input);
        console.log(`Running ${requests.length} Google Maps directions request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const result = await runDirectionsBatch(requests, client, createChargedRouteWriter(Actor, Actor));
        console.log('Google Maps directions summary:', JSON.stringify(result));

        if (result.failures.length > 0) {
            console.warn('Google Maps directions request failures:', JSON.stringify(result.failures));
        }

        if (result.chargeLimitReached) {
            await Actor.exit({
                statusMessage: `Charge limit reached after saving ${result.alternativesSaved} route alternative(s).`,
            });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. Try a smaller batch or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
