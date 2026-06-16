interface TikTokAuthor {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    [key: string]: unknown;
}

export interface TikTokMusicPost {
    aweme_id?: string;
    desc?: string;
    create_time?: number;
    digg_count?: number;
    comment_count?: number;
    share_count?: number;
    play_count?: number;
    author?: TikTokAuthor;
    [key: string]: unknown;
}

export interface TikTokMusicPostsData {
    posts?: TikTokMusicPost[];
    videos?: TikTokMusicPost[];
    aweme_list?: TikTokMusicPost[];
    hasMore?: boolean;
    has_more?: boolean;
    cursor?: string | number | null;
    max_cursor?: string | number | null;
    min_cursor?: string | number | null;
    [key: string]: unknown;
}

export interface TikTokMusicPostsResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokMusicPostsData | TikTokMusicPost[] | null;
    [key: string]: unknown;
}

export function extractPosts(data: TikTokMusicPostsResponse['data']): TikTokMusicPost[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.posts)) {
        return data.posts;
    }

    if (Array.isArray(data.videos)) {
        return data.videos;
    }

    if (Array.isArray(data.aweme_list)) {
        return data.aweme_list;
    }

    return [];
}

export function extractPagination(data: TikTokMusicPostsResponse['data']): {
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
        nextCursor: data.cursor ?? data.max_cursor ?? data.min_cursor ?? null,
    };
}
