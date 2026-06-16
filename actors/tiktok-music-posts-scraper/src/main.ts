import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokMusicPostsRequests, formatTikTokMusicPostsLookupForLog } from './request-params.js';
import type { TikTokMusicPostsInput } from './request-params.js';
import { extractPagination, extractPosts } from './response-utils.js';
import type { TikTokMusicPostsResponse } from './response-utils.js';

interface TikTokMusicPostsSummaryItem {
    request_music_id: string;
    posts_extracted: number;
    posts_returned: number;
    has_next_page: boolean;
    next_cursor: string | number | null;
    processed_time: number | null;
    charge_limit_reached: boolean;
}

interface PushDataResult {
    chargedCount?: number;
    eventChargeLimitReached?: boolean;
}

function getSavedPostCount(result: unknown, attemptedCount: number): number {
    if (!result || typeof result !== 'object') {
        return attemptedCount;
    }

    const chargedCount = (result as PushDataResult).chargedCount;
    return typeof chargedCount === 'number' ? chargedCount : attemptedCount;
}

function didReachEventChargeLimit(result: unknown): boolean {
    return Boolean(result && typeof result === 'object' && (result as PushDataResult).eventChargeLimitReached);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokMusicPostsInput>();
        if (!input) {
            throw new Error('At least one TikTok music_id is required');
        }

        const requests = buildTikTokMusicPostsRequests(input);
        console.log(`Fetching TikTok music posts for: ${formatTikTokMusicPostsLookupForLog(requests)}`);

        const client = new ScrappaClient({ apiKey });
        const results: TikTokMusicPostsSummaryItem[] = [];
        let totalPosts = 0;
        let chargeLimitReached = false;

        for (const request of requests) {
            console.log(`Fetching TikTok posts for music_id:${request.musicId}`);
            const response = await client.get<TikTokMusicPostsResponse>('/tiktok/music/posts', request.params);

            if (response.code !== undefined && response.code !== 0) {
                throw new Error(`Scrappa TikTok Music Posts API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
            }

            const posts = extractPosts(response.data);
            const pagination = extractPagination(response.data);
            let savedPosts = 0;
            let requestChargeLimitReached = false;

            if (posts.length > 0) {
                const pushResult = await Actor.pushData(posts.map((post) => ({
                    ...post,
                    request_music_id: request.musicId,
                })));
                savedPosts = getSavedPostCount(pushResult, posts.length);
                requestChargeLimitReached = didReachEventChargeLimit(pushResult);
                console.log(`Saved ${savedPosts} of ${posts.length} posts for music_id:${request.musicId}`);
            } else {
                console.log(`No posts found for music_id:${request.musicId}`);
            }

            totalPosts += savedPosts;
            results.push({
                request_music_id: request.musicId,
                posts_extracted: savedPosts,
                posts_returned: posts.length,
                has_next_page: pagination.hasNextPage,
                next_cursor: pagination.nextCursor,
                processed_time: response.processed_time ?? null,
                charge_limit_reached: requestChargeLimitReached,
            });

            if (requestChargeLimitReached) {
                chargeLimitReached = true;
                console.log('Apify event charge limit reached. Stopping before fetching additional music IDs.');
                break;
            }
        }

        const summary = {
            music_ids_processed: results.length,
            posts_extracted: totalPosts,
            charge_limit_reached: chargeLimitReached,
            results,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', summary);

        console.log('TikTok music posts extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
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
