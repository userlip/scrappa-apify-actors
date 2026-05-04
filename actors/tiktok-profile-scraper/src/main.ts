import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { normalizeTikTokProfileRecord } from './normalize-profile.js';
import { buildTikTokProfileParams, formatTikTokProfileLookupForLog } from './request-params.js';
import type { TikTokProfileInput } from './request-params.js';

interface TikTokProfile {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    signature?: string;
    follower_count?: number;
    following_count?: number;
    heart_count?: number;
    video_count?: number;
    digg_count?: number;
    verified?: boolean;
    private_account?: boolean;
    region?: string;
    language?: string;
    [key: string]: unknown;
}

interface TikTokProfileResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokProfile | TikTokProfile[] | null;
    [key: string]: unknown;
}

function extractProfile(data: TikTokProfileResponse['data']): TikTokProfile | null {
    if (!data) {
        return null;
    }

    if (Array.isArray(data)) {
        if (data.length > 1) {
            console.warn(`Scrappa returned ${data.length} profiles for a single lookup. Saving the first profile only.`);
        }
        return data[0] ?? null;
    }

    return data;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokProfileInput>();
        if (!input) {
            throw new Error('TikTok unique_id or user_id is required');
        }

        const params = buildTikTokProfileParams(input);
        console.log(`Fetching TikTok profile for: ${formatTikTokProfileLookupForLog(input)}`);

        const client = new ScrappaClient({ apiKey });
        const response = await client.get<TikTokProfileResponse>('/tiktok/user/profile', params);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Profile API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const profile = extractProfile(response.data);
        const normalizedProfile = profile ? normalizeTikTokProfileRecord(profile) : null;

        if (normalizedProfile) {
            await Actor.pushData({
                ...normalizedProfile,
                lookup_unique_id: params.unique_id ?? null,
                lookup_user_id: params.user_id ?? null,
            });
            console.log(`Found TikTok profile: ${normalizedProfile.unique_id ?? normalizedProfile.user_id ?? 'unknown'}`);
        } else {
            console.log('No profile found for the given TikTok lookup');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            profile_found: normalizedProfile !== null,
            unique_id: normalizedProfile?.unique_id ?? null,
            user_id: normalizedProfile?.user_id ?? null,
            follower_count: normalizedProfile?.follower_count ?? null,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok profile extraction completed successfully');
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
