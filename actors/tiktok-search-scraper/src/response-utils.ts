interface TikTokAuthor {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    [key: string]: unknown;
}

export interface TikTokSearchVideo {
    aweme_id?: string;
    video_id?: string;
    desc?: string;
    title?: string;
    create_time?: number;
    digg_count?: number;
    comment_count?: number;
    share_count?: number;
    play_count?: number;
    author?: TikTokAuthor;
    [key: string]: unknown;
}

export interface TikTokSearchData {
    videos?: TikTokSearchVideo[];
    posts?: TikTokSearchVideo[];
    aweme_list?: TikTokSearchVideo[];
    item_list?: TikTokSearchVideo[];
    hasMore?: boolean;
    has_more?: boolean;
    cursor?: string | number | null;
    max_cursor?: string | number | null;
    min_cursor?: string | number | null;
    [key: string]: unknown;
}

export interface TikTokSearchResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokSearchData | TikTokSearchVideo[] | null;
    [key: string]: unknown;
}

export function extractVideos(data: TikTokSearchResponse['data']): TikTokSearchVideo[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.videos)) {
        return data.videos;
    }

    if (Array.isArray(data.posts)) {
        return data.posts;
    }

    if (Array.isArray(data.aweme_list)) {
        return data.aweme_list;
    }

    if (Array.isArray(data.item_list)) {
        return data.item_list;
    }

    return [];
}

export function extractPagination(data: TikTokSearchResponse['data']): {
    hasNextPage: boolean;
    nextCursor: string | number | null;
} {
    if (!data || Array.isArray(data)) {
        return {
            hasNextPage: false,
            nextCursor: null,
        };
    }

    return {
        hasNextPage: Boolean(data.hasMore ?? data.has_more ?? false),
        // Scrappa has returned TikTok search cursors under different keys across upstream shapes.
        nextCursor: data.cursor ?? data.max_cursor ?? data.min_cursor ?? null,
    };
}
