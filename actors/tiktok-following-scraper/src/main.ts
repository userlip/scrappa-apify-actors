import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokFollowingParams, formatTikTokFollowingLookupForLog } from './request-params.js';
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

        const response = await client.get<TikTokFollowingResponse>('/tiktok/user/following', params);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Following API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const following = extractFollowing(response.data);
        const pagination = extractPagination(response.data);

        if (following.length > 0) {
            await Actor.pushData(following.map((followedUser) => ({
                ...followedUser,
                lookup_unique_id: originalLookupUniqueId,
                lookup_user_id: params.user_id ?? null,
            })));
            console.log(`Found ${following.length} followed accounts`);
        } else {
            console.log('No followed accounts found for the given TikTok lookup');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            following_extracted: following.length,
            has_next_page: pagination.hasNextPage,
            next_time: pagination.nextTime,
            processed_time: response.processed_time ?? null,
        };

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
