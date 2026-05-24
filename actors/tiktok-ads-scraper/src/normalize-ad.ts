interface TikTokAdStats {
    like_count?: number;
    comment_count?: number;
    share_count?: number;
    [key: string]: unknown;
}

interface TikTokAdAdvertiser {
    id?: string | number;
    advertiser_id?: string | number;
    account_id?: string | number;
    name?: string;
    brand_name?: string;
    account_name?: string;
    avatar?: string;
    [key: string]: unknown;
}

interface TikTokAdVideoInfo {
    play?: string;
    wmplay?: string;
    cover?: string;
    [key: string]: unknown;
}

export interface TikTokAdRecord {
    ad_id?: string | number;
    id?: string | number;
    brand_name?: string;
    advertiser_name?: string;
    advertiser_id?: string | number;
    account_id?: string | number;
    account_name?: string;
    industry?: string;
    objective?: string;
    title?: string;
    description?: string;
    ad_text?: string;
    video_url?: string;
    play_url?: string;
    media_url?: string;
    cover?: string;
    cover_uri?: string;
    cover_url?: string;
    image_url?: string;
    landing_page_url?: string;
    destination?: string;
    cta?: string;
    region?: string;
    country?: string;
    language?: string;
    category?: string;
    like_count?: number;
    like?: number;
    comment_count?: number;
    comment?: number;
    share_count?: number;
    share?: number;
    stats?: TikTokAdStats;
    advertiser?: TikTokAdAdvertiser;
    video_info?: TikTokAdVideoInfo;
    [key: string]: unknown;
}

export interface NormalizedTikTokAdRecord extends TikTokAdRecord {
    ad_id?: string;
    advertiser_id?: string;
    account_id?: string;
    advertiser_name?: string;
    account_name?: string;
    creative_text?: string;
    landing_page?: string;
    video_url?: string;
    media_urls: string[];
}

export function normalizeTikTokAdRecord(ad: TikTokAdRecord): NormalizedTikTokAdRecord {
    const advertiser = ad.advertiser;
    const stats = ad.stats;
    const videoInfo = ad.video_info;
    const videoUrl = firstString(ad.video_url, ad.play_url, ad.media_url, videoInfo?.play, videoInfo?.wmplay);
    const coverUrl = firstString(ad.cover, ad.cover_url, ad.cover_uri, ad.image_url, videoInfo?.cover);
    const landingPage = firstString(ad.landing_page_url, ad.destination);
    const mediaUrls = uniqueStrings([videoUrl, coverUrl, ad.media_url, ad.image_url, videoInfo?.play, videoInfo?.wmplay, videoInfo?.cover]);

    return {
        ...ad,
        ad_id: toOptionalString(ad.ad_id ?? ad.id),
        advertiser_id: toOptionalString(ad.advertiser_id ?? advertiser?.advertiser_id ?? advertiser?.id),
        account_id: toOptionalString(ad.account_id ?? advertiser?.account_id),
        advertiser_name: firstString(ad.advertiser_name, ad.brand_name, advertiser?.brand_name, advertiser?.name),
        account_name: firstString(ad.account_name, advertiser?.account_name, advertiser?.name),
        creative_text: firstString(ad.creative_text, ad.ad_text, ad.description, ad.title),
        landing_page: landingPage,
        video_url: videoUrl,
        cover: coverUrl,
        media_urls: mediaUrls,
        like_count: ad.like_count ?? ad.like ?? stats?.like_count,
        comment_count: ad.comment_count ?? ad.comment ?? stats?.comment_count,
        share_count: ad.share_count ?? ad.share ?? stats?.share_count,
    };
}

function toOptionalString(value: string | number | undefined | null): string | undefined {
    if (value === undefined || value === null) {
        return undefined;
    }

    return String(value);
}

function firstString(...values: unknown[]): string | undefined {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return undefined;
}

function uniqueStrings(values: unknown[]): string[] {
    return [...new Set(values.filter((value): value is string => typeof value === 'string' && value.trim() !== ''))];
}
