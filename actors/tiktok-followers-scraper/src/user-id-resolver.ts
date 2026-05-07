import type { ScrappaClient } from './shared/scrappa-client.js';

interface TikTokProfileUser {
    id?: string | number;
    [key: string]: unknown;
}

interface TikTokProfileRecord {
    user_id?: string | number;
    user?: TikTokProfileUser;
    [key: string]: unknown;
}

interface TikTokProfileResponse {
    code?: number;
    msg?: string;
    data?: TikTokProfileRecord | TikTokProfileRecord[] | null;
    [key: string]: unknown;
}

export async function resolveTikTokFollowersUserId(
    client: Pick<ScrappaClient, 'get'>,
    params: Record<string, unknown>,
): Promise<Record<string, unknown>> {
    if (typeof params.user_id === 'string' && params.user_id.trim() !== '') {
        return params;
    }

    if (typeof params.unique_id !== 'string' || params.unique_id.trim() === '') {
        return params;
    }

    const uniqueId = params.unique_id;
    console.log(`Resolving TikTok user_id for ${uniqueId}`);

    const response = await client.get<TikTokProfileResponse>('/tiktok/user/profile', { unique_id: uniqueId });
    if (response.code !== undefined && response.code !== 0) {
        throw new Error(`Scrappa TikTok Profile API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
    }

    const userId = extractUserId(response.data);
    if (!userId) {
        throw new Error(`Could not resolve TikTok user_id for ${uniqueId}`);
    }

    return {
        ...params,
        user_id: userId,
        unique_id: undefined,
    };
}

function extractUserId(data: TikTokProfileResponse['data']): string | null {
    const profile = Array.isArray(data) ? data[0] : data;
    const userId = profile?.user_id ?? profile?.user?.id;

    if (userId === undefined || userId === null) {
        return null;
    }

    const normalized = String(userId).trim();
    return normalized === '' ? null : normalized;
}
