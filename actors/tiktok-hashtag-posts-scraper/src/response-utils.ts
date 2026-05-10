interface TikTokAuthor {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    [key: string]: unknown;
}

export interface TikTokHashtagPost {
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

export interface TikTokHashtagPostsData {
    posts?: TikTokHashtagPost[];
    videos?: TikTokHashtagPost[];
    aweme_list?: TikTokHashtagPost[];
    hasMore?: boolean;
    has_more?: boolean;
    cursor?: string | number | null;
    max_cursor?: string | number | null;
    min_cursor?: string | number | null;
    [key: string]: unknown;
}

export interface TikTokHashtagPostsResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokHashtagPostsData | TikTokHashtagPost[] | null;
    [key: string]: unknown;
}

export interface TikTokChallenge {
    id?: string;
    challenge_id?: string;
    cha_name?: string;
    challenge_name?: string;
    view_count?: number;
    video_count?: number;
    [key: string]: unknown;
}

export interface TikTokChallengeSearchData {
    challenges?: TikTokChallenge[];
    challenge_list?: TikTokChallenge[];
    [key: string]: unknown;
}

export interface TikTokChallengeSearchResponse {
    code?: number;
    msg?: string;
    data?: TikTokChallengeSearchData | TikTokChallenge[] | null;
    [key: string]: unknown;
}

export interface TikTokChallengeSelection {
    challenge: TikTokChallenge;
    isExactMatch: boolean;
}

export function extractPosts(data: TikTokHashtagPostsResponse['data']): TikTokHashtagPost[] {
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

export function extractChallenges(data: TikTokChallengeSearchResponse['data']): TikTokChallenge[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.challenges)) {
        return data.challenges;
    }

    if (Array.isArray(data.challenge_list)) {
        return data.challenge_list;
    }

    return [];
}

export function selectChallengeForHashtag(challenges: TikTokChallenge[], hashtag: string): TikTokChallengeSelection | null {
    if (challenges.length === 0) {
        return null;
    }

    const normalizedTarget = normalizeChallengeName(hashtag);
    const exactMatch = challenges.find((challenge) => normalizeChallengeName(getChallengeName(challenge)) === normalizedTarget);

    if (exactMatch) {
        return {
            challenge: exactMatch,
            isExactMatch: true,
        };
    }

    return {
        challenge: challenges[0],
        isExactMatch: false,
    };
}

export function getChallengeId(challenge: TikTokChallenge): string | null {
    const id = challenge.challenge_id ?? challenge.id;
    return typeof id === 'string' && id.trim() !== '' ? id.trim() : null;
}

export function getChallengeName(challenge: TikTokChallenge): string {
    const name = challenge.challenge_name ?? challenge.cha_name;
    return typeof name === 'string' ? name.trim() : '';
}

function normalizeChallengeName(value: string): string {
    return value.trim().replace(/^#/, '').toLowerCase();
}

export function extractPagination(data: TikTokHashtagPostsResponse['data']): {
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
