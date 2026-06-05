import { Actor } from 'apify';
import {
    buildGoogleTrendsAutocompleteParams,
    describeGoogleTrendsAutocompleteRequest,
} from './request-params.js';
import type { GoogleTrendsAutocompleteInput } from './request-params.js';
import { buildAutocompleteDatasetItems } from './response-utils.js';
import type { GoogleTrendsAutocompleteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const SUGGESTION_RESULT_CHARGE_EVENT = 'suggestion-result';

async function writeOutput(
    response: GoogleTrendsAutocompleteResponse,
    datasetItems: Record<string, unknown>[],
    savedSuggestionCount: number,
    chargeLimitReached: boolean,
): Promise<void> {
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', {
        search_parameters: response.search_parameters ?? null,
        suggestion_count: datasetItems.length,
        saved_suggestion_count: savedSuggestionCount,
        charge_limit_reached: chargeLimitReached,
        raw_response_omitted: chargeLimitReached,
        response_time_ms: response.response_time_ms ?? null,
        raw_response: chargeLimitReached ? null : response,
    });
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleTrendsAutocompleteInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleTrendsAutocompleteParams(input);
        console.log(`Fetching Google Trends autocomplete suggestions for ${describeGoogleTrendsAutocompleteRequest(params)}`);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });

        const response = await client.get<GoogleTrendsAutocompleteResponse>('/google-trends/autocomplete', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const datasetItems = buildAutocompleteDatasetItems(response, params);
        let savedSuggestionCount = 0;

        if (datasetItems.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(datasetItems, SUGGESTION_RESULT_CHARGE_EVENT);
                savedSuggestionCount = chargeResult.chargedCount;
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                    const statusMessage = 'Charge limit reached before saving all Google Trends autocomplete suggestion results.';
                    console.log(statusMessage, JSON.stringify({
                        event: SUGGESTION_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: datasetItems.length,
                    }));
                    await writeOutput(response, datasetItems, savedSuggestionCount, true);
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(datasetItems);
                savedSuggestionCount = datasetItems.length;
            }
        } else {
            console.log('No Google Trends autocomplete suggestions found for this request');
        }

        await writeOutput(response, datasetItems, savedSuggestionCount, false);

        console.log('Google Trends autocomplete scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            suggestion_results: datasetItems.length,
            response_time_ms: response.response_time_ms ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Trends autocomplete request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific keyword or run the request again.`
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
