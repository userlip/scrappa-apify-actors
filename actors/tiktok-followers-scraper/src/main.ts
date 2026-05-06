import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokFollowersParams, formatTikTokFollowersLookupForLog } from './request-params.js';
import type { TikTokFollowersInput } from './request-params.js';
import { extractProfileUserId } from './profile-utils.js';
import type { TikTokProfileResponse } from './profile-utils.js';
import { extractFollowers, extractPagination } from './response-utils.js';
import type { TikTokFollowersResponse } from './response-utils.js';

async function resolveFollowersParams(
    client: ScrappaClient,
    params: Record<string, unknown>,
): Promise<Record<string, unknown>> {
    if (!params.unique_id || params.user_id) {
        return params;
    }

    console.log(`Resolving TikTok user_id for ${String(params.unique_id)}`);
    const profileResponse = await client.get<TikTokProfileResponse>('/tiktok/user/profile', {
        unique_id: params.unique_id,
    });

    if (profileResponse.code !== undefined && profileResponse.code !== 0) {
        throw new Error(`Scrappa TikTok Profile API returned code ${profileResponse.code}: ${profileResponse.msg ?? 'Unknown error'}`);
    }

    const userId = extractProfileUserId(profileResponse.data);
    if (!userId) {
        throw new Error(`Could not resolve TikTok user_id for ${String(params.unique_id)}`);
    }

    return {
        ...params,
        user_id: userId,
    };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokFollowersInput>();
        if (!input) {
            throw new Error('TikTok unique_id or user_id is required');
        }

        const params = buildTikTokFollowersParams(input);
        console.log(`Fetching TikTok followers for: ${formatTikTokFollowersLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey });
        const requestParams = await resolveFollowersParams(client, params);
        const response = await client.get<TikTokFollowersResponse>('/tiktok/user/followers', requestParams);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Followers API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const followers = extractFollowers(response.data);
        const pagination = extractPagination(response.data);

        if (followers.length > 0) {
            await Actor.pushData(followers.map((follower) => ({
                ...follower,
                lookup_unique_id: params.unique_id ?? null,
                lookup_user_id: requestParams.user_id ?? params.user_id ?? null,
            })));
            console.log(`Found ${followers.length} followers`);
        } else {
            console.log('No followers found for the given TikTok lookup');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            followers_extracted: followers.length,
            has_next_page: pagination.hasNextPage,
            next_time: pagination.nextTime,
            lookup_unique_id: params.unique_id ?? null,
            lookup_user_id: requestParams.user_id ?? params.user_id ?? null,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok followers extraction completed successfully');
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
