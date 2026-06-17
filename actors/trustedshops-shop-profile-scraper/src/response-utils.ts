export interface TrustedShopsShopProfileResponse {
    response?: {
        data?: {
            shop?: TrustedShopsShopProfileData;
            [key: string]: unknown;
        };
        responseInfo?: Record<string, unknown>;
        [key: string]: unknown;
    };
    data?: TrustedShopsShopProfileData;
    shop?: TrustedShopsShopProfileData;
    meta?: Record<string, unknown>;
    metaData?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface TrustedShopsShopProfileData {
    tsID?: string;
    tsid?: string;
    tsId?: string;
    name?: string;
    displayName?: string;
    accountName?: string;
    shopName?: string;
    url?: string;
    shopUrl?: string;
    website?: string;
    profileUrl?: string;
    profile_url?: string;
    language?: string;
    languageCode?: string;
    languageISO2?: string;
    targetMarket?: string;
    target_market?: string;
    targetMarketISO3?: string;
    market?: string;
    rating?: number;
    averageRating?: number;
    reviewCount?: number;
    reviewsCount?: number;
    certified?: boolean;
    certificationState?: boolean;
    categories?: unknown[];
    shopCategories?: unknown[];
    reviewSummary?: Record<string, unknown>;
    ratingSummary?: Record<string, unknown>;
    metadata?: Record<string, unknown>;
    profileMetadata?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface TrustedShopsShopProfileDatasetContext {
    requestedTsid: string;
    sourceUrl?: string;
    includeRawResponse: boolean;
}

export interface TrustedShopsShopProfileOutputSummaryContext {
    requested: number;
    savedProfiles: number;
    failures: Record<string, unknown>[];
    statusMessage: string | null;
}

function isObject(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function firstObject(...values: unknown[]): Record<string, unknown> {
    for (const value of values) {
        if (isObject(value)) {
            return value;
        }
    }

    return {};
}

function firstNonEmptyString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value.trim();
        }
    }

    return null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
    }

    return null;
}

function firstBoolean(...values: unknown[]): boolean | null {
    for (const value of values) {
        if (typeof value === 'boolean') {
            return value;
        }
    }

    return null;
}

function withProtocol(value: unknown): string | null {
    if (typeof value !== 'string' || value.trim() === '') {
        return null;
    }

    const url = value.trim();
    if (/^\/\//.test(url)) {
        return `https:${url}`;
    }

    return /^https?:\/\//i.test(url) ? url : `https://${url}`;
}

function trustedShopsProfileUrl(value: unknown, tsid: string): string {
    if (typeof value === 'string' && value.trim().startsWith('/')) {
        return `https://www.trustedshops.de${value.trim()}`;
    }

    const profileUrl = withProtocol(value);
    if (profileUrl) {
        return profileUrl;
    }

    return `https://www.trustedshops.de/bewertung/info_${tsid}.html`;
}

function categoryName(category: unknown): string | undefined {
    if (typeof category === 'string' && category.trim() !== '') {
        return category.trim();
    }

    if (!isObject(category)) {
        return undefined;
    }

    return firstNonEmptyString(category.name, category.displayName, category.title) ?? undefined;
}

function categoryId(category: unknown): string | undefined {
    if (!isObject(category)) {
        return undefined;
    }

    const value = category.id ?? category.categoryId ?? category.urlPath ?? category.slug;
    if (typeof value === 'number') {
        return String(value);
    }

    return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

function getProfileData(response: TrustedShopsShopProfileResponse): TrustedShopsShopProfileData {
    return firstObject(
        response.response?.data?.shop,
        response.data?.shop,
        response.data,
        response.shop,
        response,
    ) as TrustedShopsShopProfileData;
}

export function buildTrustedShopsShopProfileDatasetItem(
    response: TrustedShopsShopProfileResponse,
    context: TrustedShopsShopProfileDatasetContext,
): Record<string, unknown> {
    const profile = getProfileData(response);
    const reviewSummary = firstObject(profile.reviewSummary, profile.ratingSummary);
    const metadata = firstObject(
        profile.metadata,
        profile.profileMetadata,
        response.response?.responseInfo,
        response.meta,
        response.metaData,
    );
    const categories = Array.isArray(profile.categories)
        ? profile.categories
        : Array.isArray(profile.shopCategories)
            ? profile.shopCategories
            : [];
    const tsid = firstNonEmptyString(profile.tsid, profile.tsID, profile.tsId, context.requestedTsid)
        ?? context.requestedTsid;

    const item: Record<string, unknown> = {
        tsid,
        requested_tsid: context.requestedTsid,
        name: firstNonEmptyString(profile.name, profile.displayName, profile.accountName, profile.shopName),
        url: withProtocol(firstNonEmptyString(profile.url, profile.shopUrl, profile.website)),
        profile_url: trustedShopsProfileUrl(firstNonEmptyString(profile.profileUrl, profile.profile_url), tsid),
        language: firstNonEmptyString(profile.language, profile.languageCode, profile.languageISO2),
        target_market: firstNonEmptyString(profile.targetMarket, profile.target_market, profile.targetMarketISO3, profile.market),
        rating: firstNumber(profile.rating, profile.averageRating, reviewSummary.rating, reviewSummary.averageRating),
        review_count: firstNumber(
            profile.reviewCount,
            profile.reviewsCount,
            reviewSummary.reviewCount,
            reviewSummary.reviewsCount,
            reviewSummary.totalReviewCount,
        ),
        certified: firstBoolean(profile.certified, profile.certificationState),
        categories,
        category_names: categories.map(categoryName).filter(Boolean).join(', '),
        category_ids: categories.map(categoryId).filter(Boolean).join(', '),
        source_url: context.sourceUrl ?? null,
        profile_metadata: Object.keys(metadata).length > 0 ? metadata : null,
    };

    if (context.includeRawResponse) {
        item.raw_response = response;
    }

    return item;
}

export function buildTrustedShopsShopProfileOutputSummary(
    context: TrustedShopsShopProfileOutputSummaryContext,
): Record<string, unknown> {
    return {
        request: {
            endpoint: '/trustedshops/shop/{tsid}',
        },
        profiles_requested: context.requested,
        profiles_saved: context.savedProfiles,
        profiles_failed: context.failures.length,
        responses_saved: context.savedProfiles,
        status_message: context.statusMessage,
        failures: context.failures,
    };
}
