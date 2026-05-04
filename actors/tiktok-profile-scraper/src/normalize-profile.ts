interface TikTokProfileUser {
    id?: string | number;
    uniqueId?: string;
    nickname?: string;
    avatarThumb?: string;
    avatarMedium?: string;
    avatarLarger?: string;
    signature?: string;
    verified?: boolean;
    privateAccount?: boolean;
    region?: string;
    language?: string;
    [key: string]: unknown;
}

interface TikTokProfileStats {
    followingCount?: number;
    followerCount?: number;
    heartCount?: number;
    videoCount?: number;
    diggCount?: number;
    [key: string]: unknown;
}

interface TikTokProfileRecord {
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
    user?: TikTokProfileUser;
    stats?: TikTokProfileStats;
    [key: string]: unknown;
}

export function normalizeTikTokProfileRecord(profile: TikTokProfileRecord): TikTokProfileRecord {
    const user = profile.user;
    const stats = profile.stats;

    return {
        ...profile,
        user_id: profile.user_id ?? toOptionalString(user?.id),
        unique_id: toOptionalUniqueId(profile.unique_id) ?? toOptionalUniqueId(user?.uniqueId),
        nickname: profile.nickname ?? user?.nickname,
        avatar: profile.avatar ?? user?.avatarLarger ?? user?.avatarMedium ?? user?.avatarThumb,
        signature: profile.signature ?? user?.signature,
        follower_count: profile.follower_count ?? stats?.followerCount,
        following_count: profile.following_count ?? stats?.followingCount,
        heart_count: profile.heart_count ?? stats?.heartCount,
        video_count: profile.video_count ?? stats?.videoCount,
        digg_count: profile.digg_count ?? stats?.diggCount,
        verified: profile.verified ?? user?.verified,
        private_account: profile.private_account ?? user?.privateAccount,
        region: profile.region ?? user?.region,
        language: profile.language ?? user?.language,
    };
}

function toOptionalString(value: string | number | undefined): string | undefined {
    if (value === undefined || value === null) {
        return undefined;
    }

    return String(value);
}

function toOptionalUniqueId(value: string | undefined): string | undefined {
    if (!value) {
        return undefined;
    }

    return value.startsWith('@') ? value : `@${value}`;
}
