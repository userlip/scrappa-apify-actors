import { Actor } from 'apify';
import {
    buildSimilarwebTrafficRequests,
    describeSimilarwebTrafficRequests,
} from './request-params.js';
import type { SimilarwebTrafficInput } from './request-params.js';
import {
    buildSimilarwebDatasetItem,
    hasSimilarwebTrafficData,
} from './response-utils.js';
import type { SimilarwebTrafficResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 2;
const DOMAIN_RESULT_CHARGE_EVENT = 'domain-result';

interface PushChargedItemResult {
    saved: boolean;
    statusMessage: string | null;
}

function getChargeLimitStatus(): string | null {
    const chargingManager = Actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(DOMAIN_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching the next Similarweb domain result; no more Scrappa requests will be made.`;
}

async function pushChargedItem(item: Record<string, unknown>): Promise<PushChargedItemResult> {
    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(item);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(item, DOMAIN_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        const saved = chargeResult.chargedCount >= 1;
        const statusMessage = saved
            ? `Charge limit reached after saving Similarweb result for ${String(item.domain)}.`
            : `Charge limit reached before saving Similarweb result for ${String(item.domain)}.`;
        console.log(statusMessage, JSON.stringify({
            event: DOMAIN_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
        }));
        return { saved, statusMessage };
    }

    return { saved: true, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<SimilarwebTrafficInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildSimilarwebTrafficRequests(input);
        console.log(`Fetching Similarweb traffic analytics for ${describeSimilarwebTrafficRequests(requests)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: Record<string, unknown>[] = [];
        let processed = 0;
        let successful = 0;
        let noData = 0;
        let statusMessage: string | null = null;

        for (const request of requests) {
            statusMessage = getChargeLimitStatus();
            if (statusMessage) {
                console.log(statusMessage, JSON.stringify({
                    event: DOMAIN_RESULT_CHARGE_EVENT,
                    processed,
                    requested: requests.length,
                }));
                break;
            }

            console.log(`Fetching Similarweb traffic analytics for ${request.domain}`);

            try {
                const response = await client.get<SimilarwebTrafficResponse>('/similarweb', {
                    domain: request.domain,
                }, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });

                if (!hasSimilarwebTrafficData(response)) {
                    const item = {
                        success: false,
                        domain: request.domain,
                        input_domain: request.inputDomain,
                        request_domain: request.domain,
                        error: 'No traffic data returned',
                    };
                    const result = await pushChargedItem(item);
                    if (!result.saved) {
                        statusMessage = result.statusMessage;
                        break;
                    }

                    processed += 1;
                    noData += 1;
                    responses.push(item);
                    if (result.statusMessage) {
                        statusMessage = result.statusMessage;
                        break;
                    }
                    console.log(`No Similarweb traffic data returned for ${request.domain}`);
                    continue;
                }

                const item = buildSimilarwebDatasetItem(response, request);
                const result = await pushChargedItem(item);
                if (!result.saved) {
                    statusMessage = result.statusMessage;
                    break;
                }

                responses.push(item);
                processed += 1;
                successful += 1;
                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } catch (error) {
                if (error instanceof ScrappaHttpError && error.status === 404) {
                    const item = {
                        success: false,
                        domain: request.domain,
                        input_domain: request.inputDomain,
                        request_domain: request.domain,
                        status_code: 404,
                        error: 'No traffic data available',
                    };
                    const result = await pushChargedItem(item);
                    if (!result.saved) {
                        statusMessage = result.statusMessage;
                        break;
                    }

                    processed += 1;
                    noData += 1;
                    responses.push(item);
                    if (result.statusMessage) {
                        statusMessage = result.statusMessage;
                        break;
                    }
                    console.log(`No Similarweb traffic data available for ${request.domain}`);
                    continue;
                }

                throw error;
            }
        }

        const output = {
            requested: requests.length,
            processed,
            successful,
            no_data: noData,
            status_message: statusMessage,
            results: responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Similarweb traffic analytics completed successfully');
        console.log('Results summary:', JSON.stringify({
            requested: output.requested,
            processed: output.processed,
            successful: output.successful,
            no_data: output.no_data,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Similarweb traffic request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer domains or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
