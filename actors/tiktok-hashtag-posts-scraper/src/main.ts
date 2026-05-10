import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokHashtagPostsParams, formatTikTokHashtagPostsLookupForLog } from './request-params.js';
import type { TikTokHashtagPostsInput } from './request-params.js';
import { extractChallenges, extractPagination, extractPosts, getChallengeId, getChallengeName, selectChallengeForHashtag } from './response-utils.js';
import type { TikTokChallengeSearchResponse, TikTokHashtagPostsResponse } from './response-utils.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokHashtagPostsInput>();
        if (!input) {
            throw new Error('TikTok challenge_id or challenge_name is required');
        }

        const params = buildTikTokHashtagPostsParams(input);
        console.log(`Fetching TikTok hashtag posts for: ${formatTikTokHashtagPostsLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey });
        const postsParams = { ...params };
        let resolvedChallengeName: string | null = typeof postsParams.challenge_name === 'string' ? postsParams.challenge_name : null;

        if (typeof postsParams.challenge_name === 'string' && !postsParams.challenge_id) {
            const searchResponse = await client.get<TikTokChallengeSearchResponse>('/tiktok/challenges/search', {
                keywords: postsParams.challenge_name,
                count: 10,
            });

            if (searchResponse.code !== undefined && searchResponse.code !== 0) {
                throw new Error(`Scrappa TikTok Challenge Search API returned code ${searchResponse.code}: ${searchResponse.msg ?? 'Unknown error'}`);
            }

            const selection = selectChallengeForHashtag(extractChallenges(searchResponse.data), postsParams.challenge_name);
            const challengeId = selection ? getChallengeId(selection.challenge) : null;
            if (!selection || !challengeId) {
                throw new Error(`Could not resolve TikTok hashtag "${postsParams.challenge_name}" to a challenge_id`);
            }

            postsParams.challenge_id = challengeId;
            resolvedChallengeName = getChallengeName(selection.challenge) || postsParams.challenge_name;
            delete postsParams.challenge_name;
            if (!selection.isExactMatch) {
                console.warn(`No exact challenge match found for hashtag "${params.challenge_name}". Using first TikTok search result: challenge_id:${challengeId}${resolvedChallengeName ? ` (${resolvedChallengeName})` : ''}`);
            }
            console.log(`Resolved hashtag to challenge_id:${challengeId}${resolvedChallengeName ? ` (${resolvedChallengeName})` : ''}`);
        }

        const response = await client.get<TikTokHashtagPostsResponse>('/tiktok/challenges/posts', postsParams);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Hashtag Posts API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const posts = extractPosts(response.data);
        const pagination = extractPagination(response.data);

        if (posts.length > 0) {
            await Actor.pushData(posts.map((post) => ({
                ...post,
                lookup_challenge_name: params.challenge_name ?? null,
                lookup_challenge_id: params.challenge_id ?? null,
                resolved_challenge_name: resolvedChallengeName,
                resolved_challenge_id: postsParams.challenge_id ?? null,
                lookup_region: params.region ?? null,
            })));
            console.log(`Found ${posts.length} posts`);
        } else {
            console.log('No posts found for the given TikTok hashtag lookup');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            posts_extracted: posts.length,
            has_next_page: pagination.hasNextPage,
            next_cursor: pagination.nextCursor,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok hashtag posts extraction completed successfully');
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
