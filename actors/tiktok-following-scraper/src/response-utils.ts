export interface TikTokFollowingUser {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    follower_count?: number;
    verified?: boolean;
    [key: string]: unknown;
}

export interface TikTokFollowingData {
    following?: TikTokFollowingUser[];
    followings?: TikTokFollowingUser[];
    users?: TikTokFollowingUser[];
    user_list?: TikTokFollowingUser[];
    hasMore?: boolean;
    has_more?: boolean;
    time?: string | number | null;
    min_time?: string | number | null;
    max_time?: string | number | null;
    [key: string]: unknown;
}

export interface TikTokFollowingResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokFollowingData | TikTokFollowingUser[] | null;
    [key: string]: unknown;
}

export function extractProfileUserId(data: unknown): string | null {
    if (!data) {
        return null;
    }

    const profile = Array.isArray(data) ? data[0] : data;
    if (
        profile
        && typeof profile === 'object'
        && 'user_id' in profile
        && typeof profile.user_id === 'string'
        && profile.user_id.trim() !== ''
    ) {
        return profile.user_id.trim();
    }

    if (
        profile
        && typeof profile === 'object'
        && 'id' in profile
        && typeof profile.id === 'string'
        && profile.id.trim() !== ''
    ) {
        return profile.id.trim();
    }

    if (
        profile
        && typeof profile === 'object'
        && 'user' in profile
        && profile.user
        && typeof profile.user === 'object'
        && 'id' in profile.user
        && typeof profile.user.id === 'string'
        && profile.user.id.trim() !== ''
    ) {
        return profile.user.id.trim();
    }

    return null;
}

export function extractFollowing(data: TikTokFollowingResponse['data']): TikTokFollowingUser[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.following)) {
        return data.following;
    }

    if (Array.isArray(data.followings)) {
        return data.followings;
    }

    if (Array.isArray(data.users)) {
        return data.users;
    }

    if (Array.isArray(data.user_list)) {
        return data.user_list;
    }

    return [];
}

export function extractPagination(data: TikTokFollowingResponse['data']): {
    hasNextPage: boolean;
    nextTime: string | number | null;
} {
    if (!data || Array.isArray(data)) {
        return {
            hasNextPage: false,
            nextTime: null,
        };
    }

    return {
        hasNextPage: Boolean(data.hasMore ?? data.has_more ?? false),
        nextTime: data.time ?? data.min_time ?? data.max_time ?? null,
    };
}
