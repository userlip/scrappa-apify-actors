export interface TikTokFollower {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    follower_count?: number;
    verified?: boolean;
    [key: string]: unknown;
}

export interface TikTokFollowersData {
    followers?: TikTokFollower[];
    users?: TikTokFollower[];
    user_list?: TikTokFollower[];
    hasMore?: boolean;
    has_more?: boolean;
    time?: string | number | null;
    [key: string]: unknown;
}

export interface TikTokFollowersResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokFollowersData | TikTokFollower[] | null;
    [key: string]: unknown;
}

export function extractFollowers(data: TikTokFollowersResponse['data']): TikTokFollower[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.followers)) {
        return data.followers;
    }

    if (Array.isArray(data.users)) {
        return data.users;
    }

    if (Array.isArray(data.user_list)) {
        return data.user_list;
    }

    return [];
}

export function extractPagination(data: TikTokFollowersResponse['data']): {
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
        nextTime: data.time ?? null,
    };
}
