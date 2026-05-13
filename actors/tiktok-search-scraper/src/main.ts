import { Actor } from 'apify';
import { buildTikTokSearchParams, formatTikTokSearchLookupForLog } from './request-params.js';
import type { TikTokSearchInput } from './request-params.js';
import { extractPagination, extractVideos } from './response-utils.js';
import type { TikTokSearchResponse, TikTokSearchVideo } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function enrichVideo(video: TikTokSearchVideo, params: Record<string, unknown>): Record<string, unknown> {
    return {
        ...video,
        request_keywords: params.keywords ?? null,
        request_region: params.region ?? null,
        request_count: params.count ?? null,
        request_cursor: params.cursor ?? null,
        request_publish_time: params.publish_time ?? null,
        request_sort_type: params.sort_type ?? null,
    };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokSearchInput>();
        if (!input) {
            throw new Error('TikTok search keywords are required');
        }

        const params = buildTikTokSearchParams(input);
        console.log(`Searching TikTok videos for: ${formatTikTokSearchLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<TikTokSearchResponse>('/tiktok/feed/search', params);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Feed Search API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const videos = extractVideos(response.data);
        const pagination = extractPagination(response.data);

        if (videos.length > 0) {
            await Actor.pushData(videos.map((video) => enrichVideo(video, params)));
            console.log(`Found ${videos.length} TikTok search results`);
        } else {
            console.log('No TikTok videos found for this search');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            videos_extracted: videos.length,
            has_next_page: pagination.hasNextPage,
            next_cursor: pagination.nextCursor,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok search scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The TikTok search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific keyword or run the request again.`
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
