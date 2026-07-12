import { Actor } from 'apify';
import { runPriceInsightsBatch } from './batch-runner.js';
import { normalizeLocations } from './request-params.js';
import type { PriceInsightsInput } from './request-params.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const requests = normalizeLocations(await Actor.getInput<PriceInsightsInput>());
        console.log(`Fetching ImmobilienScout24 price insights for ${requests.length} unique location(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const pricing = Actor.getChargingManager().getPricingInfo();
        const result = await runPriceInsightsBatch(requests, client, {
            isPayPerEvent: () => pricing.isPayPerEvent,
            pushData: async (item, eventName) => {
                if (eventName) {
                    return Actor.pushData(item, eventName);
                }

                await Actor.pushData(item);
                return { chargedCount: 0, eventChargeLimitReached: false };
            },
        });

        for (const failure of result.failures) {
            console.warn(`Price insights unavailable for ${failure.location}: ${failure.message}`);
        }

        if (!result.chargeLimitReached && result.succeeded === 0) {
            throw new Error(`No price-insights snapshots were resolved for ${requests.length} requested location(s).`);
        }

        const statusMessage = result.chargeLimitReached
            ? `Charge limit reached after ${result.succeeded} successful location snapshot(s).`
            : `Saved ${result.succeeded} of ${requests.length} requested location snapshot(s); ${result.failures.length} failed.`;
        console.log(statusMessage);
        await Actor.exit({ statusMessage });
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. A Scrappa request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s timeout.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
    }
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
