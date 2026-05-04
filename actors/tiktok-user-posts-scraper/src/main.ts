import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokUserPostsParams, formatTikTokUserPostsLookupForLog } from './request-params.js';
import type { TikTokUserPostsInput } from './request-params.js';
import { extractPagination, extractPosts } from './response-utils.js';
import type { TikTokUserPostsResponse } from './response-utils.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokUserPostsInput>();
        if (!input) {
            throw new Error('TikTok unique_id or user_id is required');
        }

        const params = buildTikTokUserPostsParams(input);
        console.log(`Fetching TikTok user posts for: ${formatTikTokUserPostsLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey });
        const response = await client.get<TikTokUserPostsResponse>('/tiktok/user/posts', params);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok User Posts API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const posts = extractPosts(response.data);
        const pagination = extractPagination(response.data);

        if (posts.length > 0) {
            await Actor.pushData(posts.map((post) => ({
                ...post,
                lookup_unique_id: params.unique_id ?? null,
                lookup_user_id: params.user_id ?? null,
            })));
            console.log(`Found ${posts.length} posts`);
        } else {
            console.log('No posts found for the given TikTok lookup');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            posts_extracted: posts.length,
            has_next_page: pagination.hasNextPage,
            next_cursor: pagination.nextCursor,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok user posts extraction completed successfully');
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
