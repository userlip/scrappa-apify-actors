import { Actor } from 'apify';
import { CHALLENGE_DETAIL_CHARGE_EVENT, runChallengeDetailsBatch } from './batch-runner.js';
import { buildTikTokChallengeDetailsRequests } from './request-params.js';
import type { TikTokChallengeDetailsInput } from './request-params.js';
import type { TikTokChallengeDetailsResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function getChargeableCapacity(): number {
    const manager = Actor.getChargingManager();
    return manager.getPricingInfo().isPayPerEvent
        ? manager.calculateMaxEventChargeCountWithinLimit(CHALLENGE_DETAIL_CHARGE_EVENT)
        : Infinity;
}

async function saveChallengeDetail(item: Record<string, unknown>): Promise<{ savedCount: number; chargeLimitReached: boolean }> {
    const isPayPerEvent = Actor.getChargingManager().getPricingInfo().isPayPerEvent;
    if (!isPayPerEvent) {
        await Actor.pushData(item);
        return { savedCount: 1, chargeLimitReached: false };
    }
    const result = await Actor.pushData(item, CHALLENGE_DETAIL_CHARGE_EVENT);
    return { savedCount: Math.min(result.chargedCount, 1), chargeLimitReached: result.eventChargeLimitReached || result.chargedCount < 1 };
}

async function main(): Promise<void> {
    await Actor.init();
    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        const input = await Actor.getInput<TikTokChallengeDetailsInput>();
        if (!input) throw new Error('At least one TikTok challenge name or challenge ID is required');

        const requests = buildTikTokChallengeDetailsRequests(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const summary = await runChallengeDetailsBatch(requests, {
            getCapacity: getChargeableCapacity,
            fetch: (request) => client.get<TikTokChallengeDetailsResponse>('/tiktok/challenges/details', request.params),
            save: saveChallengeDetail,
        });
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', summary);
        console.log('TikTok challenge details completed:', JSON.stringify(summary));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }
    await Actor.exit();
}

main().catch((error) => {
    console.error('Actor failed: ' + (error instanceof Error ? error.message : String(error)));
    process.exitCode = 1;
});
