import { Actor } from 'apify';
import { buildGoogleHotelsAutocompleteRequest, paramsForQuery } from './request-params.js';
import type { GoogleHotelsAutocompleteInput } from './request-params.js';
import { buildSuggestionDatasetItems } from './response-utils.js';
import type { GoogleHotelsAutocompleteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 30000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const RESULT_CHARGE_EVENT = 'hotel-suggestion-result';

interface QueryFailure {
    query: string;
    error: string;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleHotelsAutocompleteInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const request = buildGoogleHotelsAutocompleteRequest(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const chargingManager = Actor.getChargingManager();
        const { isPayPerEvent } = chargingManager.getPricingInfo();
        const failures: QueryFailure[] = [];
        let completedQueries = 0;
        let suggestionCount = 0;
        let savedSuggestionCount = 0;
        let chargeLimitReached = false;

        console.log(`Fetching Google Hotels suggestions for ${request.queries.length} unique query or queries`);

        for (const query of request.queries) {
            const chargeableCount = isPayPerEvent
                ? chargingManager.calculateMaxEventChargeCountWithinLimit(RESULT_CHARGE_EVENT)
                : Infinity;
            if (chargeableCount <= 0) {
                chargeLimitReached = true;
                console.log(`Charge limit reached after saving ${savedSuggestionCount} suggestion result(s)`);
                break;
            }

            const params = paramsForQuery(query, request.commonParams);

            try {
                const response = await client.get<GoogleHotelsAutocompleteResponse>(
                    '/google-hotels/autocomplete',
                    params,
                    { attempts: SCRAPPA_MAX_ATTEMPTS },
                );
                const items = buildSuggestionDatasetItems(response, query, request.commonParams);
                suggestionCount += items.length;
                completedQueries++;

                if (items.length === 0) {
                    console.log(`No suggestions found for query "${query}"`);
                    continue;
                }

                if (isPayPerEvent) {
                    const itemsToSave = items.slice(0, chargeableCount);
                    const result = await Actor.pushData(itemsToSave, RESULT_CHARGE_EVENT);
                    savedSuggestionCount += itemsToSave.length;
                    if (itemsToSave.length < items.length || result.eventChargeLimitReached) {
                        chargeLimitReached = true;
                        console.log(`Charge limit reached after saving ${savedSuggestionCount} suggestion result(s)`);
                        break;
                    }
                } else {
                    await Actor.pushData(items);
                    savedSuggestionCount += items.length;
                }
            } catch (error) {
                const rawMessage = error instanceof Error ? error.message : String(error);
                const message = error instanceof ScrappaTimeoutError
                    ? `${rawMessage}. The request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout.`
                    : rawMessage;
                failures.push({ query, error: message });
                console.error(`Query "${query}" failed: ${message}`);
            }
        }

        if (completedQueries === 0 && failures.length > 0) {
            throw new Error(`All queries failed: ${failures.map(({ query, error }) => `${query}: ${error}`).join('; ')}`);
        }

        const summary = {
            requested_queries: request.queries.length,
            completed_queries: completedQueries,
            failed_queries: failures,
            suggestions_found: suggestionCount,
            suggestions_saved: savedSuggestionCount,
            charge_event: RESULT_CHARGE_EVENT,
            charge_limit_reached: chargeLimitReached,
        };
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', summary);

        console.log('Google Hotels autocomplete completed:', JSON.stringify(summary));
        const statusMessage = chargeLimitReached
            ? `Charge limit reached after ${savedSuggestionCount} suggestion results.`
            : `Saved ${savedSuggestionCount} suggestion results from ${completedQueries} queries${failures.length > 0 ? `; ${failures.length} failed` : ''}.`;
        await Actor.exit({ statusMessage });
        return;
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
    }
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
