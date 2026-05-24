import { Actor } from 'apify';
import {
    buildGoogleTrendsAutocompleteParams,
    buildGoogleTrendsRelatedQueriesParams,
    describeGoogleTrendsRelatedQueriesRequest,
    shouldIncludeAutocomplete,
} from './request-params.js';
import type { GoogleTrendsRelatedQueriesInput } from './request-params.js';
import { buildRelatedDatasetItems } from './response-utils.js';
import type { GoogleTrendsRelatedResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const RELATED_RESULT_CHARGE_EVENT = 'related-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleTrendsRelatedQueriesInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleTrendsRelatedQueriesParams(input);
        const includeAutocomplete = shouldIncludeAutocomplete(input);
        console.log(`Fetching Google Trends related queries for ${describeGoogleTrendsRelatedQueriesRequest(params)}`);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });

        const response = await client.get<GoogleTrendsRelatedResponse>('/google-trends/related', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const datasetItems = buildRelatedDatasetItems(response, params);

        const autocompleteResponse = includeAutocomplete
            ? await client.get<Record<string, unknown>>('/google-trends/autocomplete', buildGoogleTrendsAutocompleteParams(params), {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            })
            : null;

        if (datasetItems.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(datasetItems, RELATED_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                    const statusMessage = 'Charge limit reached before saving all Google Trends related query results.';
                    console.log(statusMessage, JSON.stringify({
                        event: RELATED_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: datasetItems.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(datasetItems);
            }
        } else {
            console.log('No Google Trends related queries or topics found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', {
            search_parameters: response.search_parameters ?? null,
            related_query_count: datasetItems.filter((item) => item.result_kind === 'query').length,
            related_topic_count: datasetItems.filter((item) => item.result_kind === 'topic').length,
            response_time_ms: response.response_time_ms ?? null,
            autocomplete: autocompleteResponse,
            raw_response: response,
        });

        console.log('Google Trends related queries scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            related_results: datasetItems.length,
            autocomplete_included: includeAutocomplete,
            response_time_ms: response.response_time_ms ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Trends related queries request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again.`
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
