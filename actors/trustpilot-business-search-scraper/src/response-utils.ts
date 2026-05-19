import type { TrustpilotSearchType } from './request-params.js';

export interface TrustpilotBusinessUnit {
    id?: string;
    businessUnitId?: string;
    displayName?: string;
    name?: string;
    identifyingName?: string;
    websiteUrl?: string;
    contact?: {
        website?: string | null;
        email?: string | null;
        phone?: string | null;
        [key: string]: unknown;
    };
    score?: {
        trustScore?: number;
        stars?: number;
        [key: string]: unknown;
    };
    trustScore?: number;
    stars?: number;
    numberOfReviews?: number;
    totalNumberOfReviews?: number;
    profileImageUrl?: string | null;
    logoUrl?: string | null;
    logo?: string | null;
    countryCode?: string;
    location?: {
        country?: string | null;
        city?: string | null;
        [key: string]: unknown;
    };
    address?: Record<string, unknown>;
    verification?: Record<string, unknown>;
    verified?: boolean;
    email?: string | null;
    phone?: string | null;
    isVerified?: boolean;
    isBusinessClaimed?: boolean;
    isClaimed?: boolean;
    categories?: TrustpilotCategory[];
    [key: string]: unknown;
}

export interface TrustpilotCategory {
    id?: string;
    name?: string;
    displayName?: string;
    categoryId?: string;
    slug?: string;
    [key: string]: unknown;
}

export interface TrustpilotBusinessSearchResponse {
    businessUnits?: TrustpilotBusinessUnit[];
    pageProps?: {
        businessUnits?: {
            businesses?: TrustpilotBusinessUnit[];
            totalPages?: number;
            totalHits?: number;
            [key: string]: unknown;
        };
        pagination?: {
            current_page?: number | string;
            total_pages?: number;
            total_count?: number;
            per_page?: number;
            has_next_page?: boolean;
            has_previous_page?: boolean;
            [key: string]: unknown;
        };
        categoryDisplayName?: string;
        [key: string]: unknown;
    };
    pagination?: {
        currentPage?: number;
        totalPages?: number;
        totalResults?: number;
        perPage?: number;
        pageSize?: number;
        [key: string]: unknown;
    };
    searchMode?: string;
    meta?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface TrustpilotBusinessDatasetContext {
    searchType: TrustpilotSearchType;
    params: Record<string, unknown>;
    response: TrustpilotBusinessSearchResponse;
}

export function getTrustpilotBusinesses(response: TrustpilotBusinessSearchResponse): TrustpilotBusinessUnit[] {
    if (Array.isArray(response.businessUnits)) {
        return response.businessUnits;
    }

    const categoryBusinesses = response.pageProps?.businessUnits?.businesses;
    if (Array.isArray(categoryBusinesses)) {
        return categoryBusinesses;
    }

    return [];
}

function withProtocol(url: unknown): string | null {
    if (typeof url !== 'string' || url.trim() === '') {
        return null;
    }

    const trimmed = url.trim();
    if (/^\/\//.test(trimmed)) {
        return `https:${trimmed}`;
    }

    return /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
}

function profileUrl(business: TrustpilotBusinessUnit): string | null {
    const identifyingName = business.identifyingName;
    if (typeof identifyingName === 'string' && identifyingName.trim() !== '') {
        return `https://www.trustpilot.com/review/${identifyingName.trim()}`;
    }

    return null;
}

function businessId(business: TrustpilotBusinessUnit): string | null {
    return business.businessUnitId ?? business.id ?? null;
}

function businessName(business: TrustpilotBusinessUnit): string | null {
    return business.displayName ?? business.name ?? null;
}

function trustScore(business: TrustpilotBusinessUnit): number | null {
    return business.trustScore ?? business.score?.trustScore ?? null;
}

function stars(business: TrustpilotBusinessUnit): number | null {
    return business.stars ?? business.score?.stars ?? null;
}

function reviewCount(business: TrustpilotBusinessUnit): number | null {
    return business.numberOfReviews ?? business.totalNumberOfReviews ?? null;
}

function categoryName(category: TrustpilotCategory): string | undefined {
    return category.displayName ?? category.name;
}

function categorySlug(category: TrustpilotCategory): string | undefined {
    return category.slug ?? category.categoryId ?? category.id;
}

function totalPages(context: TrustpilotBusinessDatasetContext): number | null {
    return context.response.pagination?.totalPages
        ?? context.response.pageProps?.pagination?.total_pages
        ?? context.response.pageProps?.businessUnits?.totalPages
        ?? null;
}

function totalResults(context: TrustpilotBusinessDatasetContext): number | null {
    return context.response.pagination?.totalResults
        ?? context.response.pageProps?.pagination?.total_count
        ?? context.response.pageProps?.businessUnits?.totalHits
        ?? null;
}

function perPage(context: TrustpilotBusinessDatasetContext): number | null {
    return context.response.pagination?.perPage
        ?? context.response.pagination?.pageSize
        ?? context.response.pageProps?.pagination?.per_page
        ?? null;
}

export function hasNextPage(response: TrustpilotBusinessSearchResponse, page: number): boolean {
    if (typeof response.pageProps?.pagination?.has_next_page === 'boolean') {
        return response.pageProps.pagination.has_next_page;
    }

    const companyTotalPages = response.pagination?.totalPages;
    if (typeof companyTotalPages === 'number') {
        return page < companyTotalPages;
    }

    const pagePropsTotalPages = response.pageProps?.pagination?.total_pages;
    if (typeof pagePropsTotalPages === 'number') {
        return page < pagePropsTotalPages;
    }

    const categoryTotalPages = response.pageProps?.businessUnits?.totalPages;
    if (typeof categoryTotalPages === 'number') {
        return page < categoryTotalPages;
    }

    console.warn(`No pagination data found in response; stopping after page ${page}`);
    return false;
}

export function buildTrustpilotBusinessDatasetItem(
    business: TrustpilotBusinessUnit,
    context: TrustpilotBusinessDatasetContext,
): Record<string, unknown> {
    const categories = Array.isArray(business.categories) ? business.categories : [];
    const website = business.contact?.website ?? business.websiteUrl;
    const address = business.address ?? {};

    return {
        ...business,
        business_id: businessId(business),
        business_name: businessName(business),
        identifying_name: business.identifyingName ?? null,
        website_url: withProtocol(website),
        profile_url: profileUrl(business),
        logo_url: withProtocol(business.logo ?? business.logoUrl ?? business.profileImageUrl),
        trust_score: trustScore(business),
        stars: stars(business),
        review_count: reviewCount(business),
        is_verified: business.isVerified ?? business.verified ?? null,
        is_claimed: business.isClaimed ?? business.isBusinessClaimed ?? null,
        country_code: business.countryCode ?? address.countryCode ?? null,
        country: business.location?.country ?? address.country ?? null,
        city: business.location?.city ?? address.city ?? null,
        address,
        email: business.contact?.email ?? business.email ?? null,
        phone: business.contact?.phone ?? business.phone ?? null,
        category_names: categories.map(categoryName).filter(Boolean).join(', '),
        category_slugs: categories.map(categorySlug).filter(Boolean).join(', '),
        request_search_type: context.searchType,
        request_query: context.params.query ?? null,
        request_category: context.params.category ?? null,
        request_country: context.params.country ?? null,
        request_page: context.params.page ?? null,
        request_locale: context.params.locale ?? null,
        request_min_rating: context.params.min_rating ?? null,
        request_min_review_count: context.params.min_review_count ?? null,
        request_sort: context.params.sort ?? null,
        request_claimed: context.params.claimed ?? null,
        request_trustscore: context.params.trustscore ?? null,
        total_results: totalResults(context),
        total_pages: totalPages(context),
        per_page: perPage(context),
        search_mode: context.response.searchMode ?? null,
        category_display_name: context.response.pageProps?.categoryDisplayName ?? null,
        response_source: context.response.meta?.source ?? null,
        scraped_at: context.response.meta?.scraped_at ?? null,
    };
}
