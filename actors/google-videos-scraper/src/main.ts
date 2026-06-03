import { Actor } from 'apify';
import { buildGoogleVideosParamList, describeGoogleVideosRequest } from './request-params.js';
import type { GoogleVideosInput } from './request-params.js';
import { enrichResult, extractVideoResults } from './response-utils.js';
import type { GoogleVideosResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const VIDEO_RESULT_CHARGE_EVENT = 'apify-default-dataset-item';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleVideosInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const paramList = buildGoogleVideosParamList(input);
        console.log(`Running ${paramList.length} Google Videos request${paramList.length === 1 ? '' : 's'}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const keepRawResponse = paramList.length === 1;
        let singleResponse: GoogleVideosResponse | null = null;
        const requestSummaries: Array<{ request: Record<string, unknown>; video_results: number }> = [];
        let totalVideoResults = 0;
        let totalFoundInVideos = 0;
        let totalShortVideos = 0;
        let totalRelatedSearches = 0;
        let hasPagination = false;
        let hasScrappaPagination = false;

        for (const params of paramList) {
            console.log(`Fetching Google Videos for ${describeGoogleVideosRequest(params)}`);
            const response = await client.get<GoogleVideosResponse>('/google/videos', params);
            if (keepRawResponse) {
                singleResponse = response;
            }
            const videoResults = extractVideoResults(response);
            const datasetItems = videoResults.map((result) => enrichResult(result, params));
            totalVideoResults += videoResults.length;
            totalFoundInVideos += response.found_in_videos?.length ?? 0;
            totalShortVideos += response.short_videos?.length ?? 0;
            totalRelatedSearches += response.related_searches?.length ?? 0;
            hasPagination ||= !!response.pagination;
            hasScrappaPagination ||= !!response.scrappa_pagination;

            if (datasetItems.length > 0) {
                const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
                if (isPayPerEvent) {
                    const chargeResult = await Actor.pushData(datasetItems, VIDEO_RESULT_CHARGE_EVENT);
                    if (chargeResult.eventChargeLimitReached) {
                        const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${datasetItems.length} Google Videos results; OUTPUT was not written.`;
                        console.log(statusMessage, JSON.stringify({
                            event: VIDEO_RESULT_CHARGE_EVENT,
                            charged_count: chargeResult.chargedCount,
                            requested_count: datasetItems.length,
                        }));
                        await Actor.exit({ statusMessage });
                        return;
                    }
                } else {
                    await Actor.pushData(datasetItems);
                }

                console.log(`Found ${videoResults.length} video results`);
            } else {
                console.log('No Google Videos results found for this request');
            }

            requestSummaries.push({ request: params, video_results: videoResults.length });
        }

        const store = await Actor.openKeyValueStore();
        if (keepRawResponse) {
            await store.setValue('OUTPUT', singleResponse);
        } else {
            await store.setValue('OUTPUT', {
                requests: requestSummaries,
                video_results: totalVideoResults,
            });
        }

        const summary = {
            requests: paramList.length,
            video_results: totalVideoResults,
            found_in_videos: totalFoundInVideos,
            short_videos: totalShortVideos,
            related_searches: totalRelatedSearches,
            has_pagination: hasPagination,
            has_scrappa_pagination: hasScrappaPagination,
        };

        console.log('Google Videos scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Google Videos request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific query or run the request again.`
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
