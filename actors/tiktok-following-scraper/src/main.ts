import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import {
    SCRAPPA_TIKTOK_FOLLOWING_PAGE_SIZE,
    buildTikTokFollowingParams,
    formatTikTokFollowingLookupForLog,
} from './request-params.js';
import type { TikTokFollowingInput } from './request-params.js';
import { extractFollowing, extractPagination, extractProfileUserId } from './response-utils.js';
import type { TikTokFollowingResponse } from './response-utils.js';

interface TikTokProfileResponse {
    code?: number;
    msg?: string;
    data?: unknown;
    [key: string]: unknown;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokFollowingInput>();
        if (!input) {
            throw new Error('TikTok unique_id or user_id is required');
        }

        const params = buildTikTokFollowingParams(input);
        const requestedCount = typeof params.count === 'number' ? params.count : 10;
        const originalLookupUniqueId = typeof params.unique_id === 'string' ? params.unique_id : null;
        console.log(`Fetching TikTok following for: ${formatTikTokFollowingLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey });
        if (typeof params.unique_id === 'string' && !params.user_id) {
            console.log(`Resolving TikTok username ${params.unique_id} to numeric user_id`);
            const profileResponse = await client.get<TikTokProfileResponse>('/tiktok/user/profile', { unique_id: params.unique_id });

            if (profileResponse.code !== undefined && profileResponse.code !== 0) {
                throw new Error(`Scrappa TikTok Profile API returned code ${profileResponse.code}: ${profileResponse.msg ?? 'Unknown error'}`);
            }

            const userId = extractProfileUserId(profileResponse.data);
            if (!userId) {
                throw new Error(`Could not resolve ${params.unique_id} to a TikTok user_id`);
            }

            params.user_id = userId;
            delete params.unique_id;
        }

        let latestResponse: TikTokFollowingResponse | null = null;
        let latestPagination = { hasNextPage: false, nextTime: null as string | number | null };
        let nextTime = params.time;
        let followingExtracted = 0;
        let pagesFetched = 0;

        while (followingExtracted < requestedCount) {
            const currentTime = nextTime;
            const remainingCount = requestedCount - followingExtracted;
            const pageCount = Math.min(SCRAPPA_TIKTOK_FOLLOWING_PAGE_SIZE, remainingCount);
            const pageParams: Record<string, unknown> = {
                ...params,
                count: pageCount,
            };

            if (currentTime !== undefined) {
                pageParams.time = currentTime;
            }

            console.log(`Fetching TikTok following page ${pagesFetched + 1} (${pageCount} requested)`);
            const response = await client.get<TikTokFollowingResponse>('/tiktok/user/following', pageParams);
            latestResponse = response;
            pagesFetched += 1;

            if (response.code !== undefined && response.code !== 0) {
                throw new Error(`Scrappa TikTok Following API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
            }

            const following = extractFollowing(response.data);
            latestPagination = extractPagination(response.data);

            if (following.length > 0) {
                const datasetItems = following.slice(0, remainingCount).map((followedUser) => ({
                    ...followedUser,
                    lookup_unique_id: originalLookupUniqueId,
                    lookup_user_id: params.user_id ?? null,
                }));

                await Actor.pushData(datasetItems);
                followingExtracted += datasetItems.length;
                console.log(`Found ${datasetItems.length} followed accounts on page ${pagesFetched}`);
            }

            if (
                following.length === 0
                || !latestPagination.hasNextPage
                || latestPagination.nextTime === null
                || (currentTime !== undefined && latestPagination.nextTime === currentTime)
            ) {
                break;
            }

            nextTime = latestPagination.nextTime;
        }

        if (followingExtracted === 0) {
            console.log('No followed accounts found for the given TikTok lookup');
        } else {
            console.log(`Found ${followingExtracted} followed accounts`);
        }

        const store = await Actor.openKeyValueStore();

        const summary = {
            following_extracted: followingExtracted,
            requested_count: requestedCount,
            pages_fetched: pagesFetched,
            has_next_page: latestPagination.hasNextPage,
            next_time: latestPagination.nextTime,
            processed_time: latestResponse?.processed_time ?? null,
        };

        await store.setValue('OUTPUT', requestedCount <= SCRAPPA_TIKTOK_FOLLOWING_PAGE_SIZE && latestResponse ? latestResponse : summary);

        console.log('TikTok following extraction completed successfully');
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
