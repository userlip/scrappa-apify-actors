import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import {
    buildTrustedShopsShopProfilePlan,
    describeTrustedShopsShopProfileRequest,
} from './request-params.js';
import type { TrustedShopsShopProfileInput } from './request-params.js';
import {
    buildTrustedShopsShopProfileDatasetItem,
    buildTrustedShopsShopProfileOutputSummary,
} from './response-utils.js';
import type { TrustedShopsShopProfileResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function formatErrorMessage(error: unknown): string {
    const rawMessage = error instanceof Error ? error.message : String(error);
    return error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The TrustedShops shop profile request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer TSIDs or run the request again.`
        : rawMessage;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TrustedShopsShopProfileInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustedShopsShopProfilePlan(input);
        console.log(`Fetching TrustedShops shop profiles for ${describeTrustedShopsShopProfileRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const failures: Record<string, unknown>[] = [];
        let savedProfiles = 0;
        let statusMessage: string | null = null;

        for (const request of plan.requests) {
            if (!request.tsid) {
                failures.push({
                    source_url: request.source_url ?? null,
                    error: request.validation_error ?? 'Invalid TrustedShops profile input',
                });
                console.warn(`Skipping invalid TrustedShops profile input: ${request.validation_error ?? request.source_url ?? 'unknown input'}`);
                continue;
            }

            console.log(`Fetching TrustedShops shop profile for ${request.tsid}`);

            try {
                const response = await client.get<TrustedShopsShopProfileResponse>(
                    `/trustedshops/shop/${encodeURIComponent(request.tsid)}`,
                    {},
                    { attempts: SCRAPPA_MAX_ATTEMPTS },
                );
                const item = buildTrustedShopsShopProfileDatasetItem(response, {
                    requestedTsid: request.tsid,
                    sourceUrl: request.source_url,
                    includeRawResponse: plan.includeRawResponse,
                });
                const result = await pushChargedItems({
                    isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                    pushData: (items, eventName) => eventName === undefined
                        ? Actor.pushData(items)
                        : Actor.pushData(items, eventName),
                }, [item]);
                savedProfiles += result.savedCount;
                console.log(`Saved ${result.savedCount} TrustedShops shop profile result(s) for ${request.tsid}`);

                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } catch (error) {
                const message = formatErrorMessage(error);
                failures.push({
                    tsid: request.tsid,
                    source_url: request.source_url ?? null,
                    error: message,
                });
                console.error(`Failed to fetch TrustedShops shop profile for ${request.tsid}: ${message}`);
            }
        }

        if (!statusMessage && failures.length > 0) {
            statusMessage = `${failures.length} of ${plan.requests.length} TrustedShops shop profile request(s) failed.`;
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', buildTrustedShopsShopProfileOutputSummary({
            requested: plan.requests.length,
            savedProfiles,
            failures,
            statusMessage,
        }));

        console.log('TrustedShops shop profile extraction completed');
        console.log('Results summary:', JSON.stringify({
            profiles_requested: plan.requests.length,
            profiles_saved: savedProfiles,
            profiles_failed: failures.length,
        }));

        if (savedProfiles === 0 && failures.length > 0) {
            await Actor.fail(statusMessage ?? 'No TrustedShops shop profiles were saved.');
            return;
        }

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const message = formatErrorMessage(error);
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
