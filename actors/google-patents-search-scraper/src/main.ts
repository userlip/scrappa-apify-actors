import { Actor } from 'apify';
import { buildGooglePatentsSearchParams, describeGooglePatentsSearchRequest } from './request-params.js';
import type { GooglePatentsSearchInput } from './request-params.js';
import { enrichResult, extractPatentResults, extractPatentSearchData, limitPatentSearchResponse } from './response-utils.js';
import type { GooglePatentsSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_REQUEST_ATTEMPTS = 3;
const PATENT_RESULT_CHARGE_EVENT = 'apify-default-dataset-item';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GooglePatentsSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGooglePatentsSearchParams(input);
        console.log(`Fetching Google Patents for ${describeGooglePatentsSearchRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GooglePatentsSearchResponse>('/google-patents/search', params, {
            attempts: SCRAPPA_REQUEST_ATTEMPTS,
        });
        const data = extractPatentSearchData(response);
        const patents = extractPatentResults(response);

        if (patents.length > 0) {
            const datasetItems = patents.map((result) => enrichResult(result, params));
            const chargeResult = await Actor.getDefaultInstance().pushData(datasetItems);
            if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                const outputResponse = limitPatentSearchResponse(response, chargeResult.chargedCount);
                const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount}/${datasetItems.length} Google Patents result(s).`;
                console.log(statusMessage, JSON.stringify({
                    event: PATENT_RESULT_CHARGE_EVENT,
                    charged_count: chargeResult.chargedCount,
                    result_count: datasetItems.length,
                }));

                const store = await Actor.openKeyValueStore();
                await store.setValue('OUTPUT', outputResponse);
                await Actor.exit({ statusMessage });
                return;
            }
            console.log(`Found ${patents.length} patent results`);
        } else {
            console.log('No Google Patents results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            patent_results: patents.length,
            total_results: data.total_results ?? null,
            total_pages: data.total_pages ?? null,
            current_page: data.current_page ?? null,
            many_results: data.many_results ?? false,
            cached: data.cached ?? false,
            stale: data.stale ?? false,
            with_pdf: patents.filter((result) => typeof result.pdf === 'string' && result.pdf !== '').length,
            with_family_status: patents.filter((result) => (result.family_status?.length ?? 0) > 0).length,
        };

        console.log('Google Patents scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Patents request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific query or run the request again.`
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
