import { Actor } from 'apify';
import {
    buildTikTokChallengeSearchRequests,
    formatTikTokChallengeSearchLookupForLog,
} from './request-params.js';
import type { TikTokChallengeSearchInput, TikTokChallengeSearchRequest } from './request-params.js';
import { extractChallenges, normalizeChallengeResult } from './response-utils.js';
import type { TikTokChallengeSearchResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const CHALLENGE_RESULT_CHARGE_EVENT = 'challenge-result';

interface PushChargedChallengesResult {
    savedCount: number;
    statusMessage: string | null;
}

function getChargeableChallengeCapacity(): number {
    const chargingManager = Actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();

    if (!isPayPerEvent) {
        return Infinity;
    }

    return chargingManager.calculateMaxEventChargeCountWithinLimit(CHALLENGE_RESULT_CHARGE_EVENT);
}

async function pushChargedChallenges(
    challenges: Record<string, unknown>[],
    request: TikTokChallengeSearchRequest,
): Promise<PushChargedChallengesResult> {
    if (challenges.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(challenges);
        return { savedCount: challenges.length, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(challenges, CHALLENGE_RESULT_CHARGE_EVENT);
    const savedCount = Math.min(chargeResult.chargedCount, challenges.length);
    if (chargeResult.eventChargeLimitReached || savedCount < challenges.length) {
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${challenges.length} TikTok challenge result(s) for keyword ${request.keyword}.`;
        console.log(statusMessage, JSON.stringify({
            event: CHALLENGE_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: challenges.length,
            saved_count: savedCount,
            keyword: request.keyword,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokChallengeSearchInput>();
        if (!input) {
            throw new Error('At least one TikTok challenge search keyword is required');
        }

        const requests = buildTikTokChallengeSearchRequests(input);
        console.log(`Searching TikTok challenges for: ${formatTikTokChallengeSearchLookupForLog(requests)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const results: Array<{
            request_keyword: string;
            challenges_returned: number;
            challenges_saved: number;
            processed_time: number | null;
            charge_limit_reached: boolean;
        }> = [];
        let savedChallenges = 0;
        let statusMessage: string | null = null;

        for (const request of requests) {
            if (getChargeableChallengeCapacity() <= 0) {
                statusMessage = `Charge limit reached before fetching TikTok challenge keyword ${request.keyword}.`;
                console.log(statusMessage, JSON.stringify({
                    event: CHALLENGE_RESULT_CHARGE_EVENT,
                    keyword: request.keyword,
                }));
                break;
            }

            console.log(`Searching TikTok challenges for keyword: ${request.keyword}`);
            const response = await client.get<TikTokChallengeSearchResponse>('/tiktok/challenges/search', request.params);

            if (response.code !== undefined && response.code !== 0) {
                throw new Error(`Scrappa TikTok Challenge Search API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
            }

            const challenges = extractChallenges(response.data);
            const datasetItems = challenges.map((challenge) => normalizeChallengeResult(challenge, {
                keyword: request.keyword,
                count: request.params.count,
            }));
            const pushResult = await pushChargedChallenges(datasetItems, request);

            savedChallenges += pushResult.savedCount;
            results.push({
                request_keyword: request.keyword,
                challenges_returned: challenges.length,
                challenges_saved: pushResult.savedCount,
                processed_time: response.processed_time ?? null,
                charge_limit_reached: pushResult.statusMessage !== null,
            });

            console.log(`Found ${challenges.length} challenge(s); saved ${pushResult.savedCount} for keyword: ${request.keyword}`);

            if (pushResult.statusMessage) {
                statusMessage = pushResult.statusMessage;
                break;
            }
        }

        const output = {
            keywords_requested: requests.length,
            keywords_completed: results.length,
            challenges_extracted: savedChallenges,
            status_message: statusMessage,
            results,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('TikTok challenge search completed successfully');
        console.log('Results summary:', JSON.stringify(output));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The TikTok challenge search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific keyword or run the request again.`
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
