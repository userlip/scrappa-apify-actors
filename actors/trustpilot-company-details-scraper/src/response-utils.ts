export interface TrustpilotCategory {
    id?: string;
    name?: string;
    displayName?: string;
    categoryId?: string;
    slug?: string;
    [key: string]: unknown;
}

export interface TrustpilotMetadata {
    source?: string | null;
    scraped_at?: string | null;
    [key: string]: unknown;
}

export interface TrustpilotCompanyDetailsData {
    basic_info?: {
        name?: string | null;
        display_name?: string | null;
        domain?: string | null;
        website?: string | null;
        website_url?: string | null;
        profile_url?: string | null;
        profileUrl?: string | null;
        identifying_name?: string | null;
        business_unit_id?: string | null;
        logo?: string | null;
        logo_url?: string | null;
        image_url?: string | null;
        is_claimed?: boolean | null;
        is_verified?: boolean | null;
        claimed?: boolean | null;
        verified?: boolean | null;
        [key: string]: unknown;
    };
    ratings?: {
        trustscore?: number | null;
        trust_score?: number | null;
        stars?: number | null;
        count?: number | null;
        review_count?: number | null;
        total_reviews?: number | null;
        [key: string]: unknown;
    };
    location?: {
        country?: string | null;
        country_code?: string | null;
        city?: string | null;
        address?: string | null;
        [key: string]: unknown;
    };
    categories?: unknown[];
    contact?: {
        website?: string | null;
        email?: string | null;
        phone?: string | null;
        [key: string]: unknown;
    };
    social_media?: Record<string, unknown> | unknown[] | null;
    metadata?: TrustpilotMetadata;
    [key: string]: unknown;
}

export interface TrustpilotCompanyDetailsResponse extends TrustpilotCompanyDetailsData {
    success?: boolean;
    message?: string;
    data?: TrustpilotCompanyDetailsData;
    meta?: TrustpilotMetadata;
}

export interface TrustpilotCompanyDetailsDatasetContext {
    companyDomain: string;
    params: Record<string, unknown>;
}

export interface TrustpilotCompanyDetailsOutputSummaryContext {
    domains: string[];
    baseParams: Record<string, unknown>;
    savedCompanies: number;
    failures: Record<string, string>[];
    statusMessage: string | null;
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

function firstNonEmptyString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value.trim();
        }
    }

    return null;
}

function trustpilotProfileUrl(value: unknown, domain: string): string {
    if (typeof value !== 'string' || value.trim() === '') {
        return `https://www.trustpilot.com/review/${domain}`;
    }

    const trimmed = value.trim();
    if (/^\/\//.test(trimmed)) {
        return withProtocol(trimmed) ?? `https://www.trustpilot.com/review/${domain}`;
    }

    if (trimmed.startsWith('/')) {
        return `https://www.trustpilot.com${trimmed}`;
    }

    return withProtocol(trimmed) ?? `https://www.trustpilot.com/review/${domain}`;
}

function categoryName(category: TrustpilotCategory): string | undefined {
    return category.displayName ?? category.name;
}

function categorySlug(category: TrustpilotCategory): string | undefined {
    return category.slug ?? category.categoryId ?? category.id;
}

function isTrustpilotCategory(value: unknown): value is TrustpilotCategory {
    return typeof value === 'object' && value !== null;
}

export function buildTrustpilotCompanyDetailsDatasetItem(
    response: TrustpilotCompanyDetailsResponse,
    context: TrustpilotCompanyDetailsDatasetContext,
): Record<string, unknown> {
    const details = response.data ?? response;
    const basicInfo = details.basic_info ?? {};
    const ratings = details.ratings ?? {};
    const location = details.location ?? {};
    const contact = details.contact ?? {};
    const metadata = details.metadata ?? {};
    const scrapeMetadata = response.meta ?? {};
    const categories = Array.isArray(details.categories)
        ? details.categories.filter(isTrustpilotCategory)
        : [];
    const domain = firstNonEmptyString(basicInfo.domain, basicInfo.identifying_name, context.companyDomain)
        ?? context.companyDomain;

    return {
        ...response,
        company_domain: domain,
        requested_company_domain: context.companyDomain,
        company_name: basicInfo.name ?? basicInfo.display_name ?? null,
        business_unit_id: basicInfo.business_unit_id ?? null,
        website_url: withProtocol(firstNonEmptyString(contact.website, basicInfo.website, basicInfo.website_url, domain)),
        profile_url: trustpilotProfileUrl(basicInfo.profile_url ?? basicInfo.profileUrl, domain),
        logo_url: withProtocol(basicInfo.logo_url ?? basicInfo.image_url ?? basicInfo.logo),
        trust_score: ratings.trustscore ?? ratings.trust_score ?? null,
        stars: ratings.stars ?? null,
        review_count: ratings.count ?? ratings.review_count ?? ratings.total_reviews ?? null,
        is_claimed: basicInfo.is_claimed ?? basicInfo.claimed ?? null,
        is_verified: basicInfo.is_verified ?? basicInfo.verified ?? null,
        country: location.country ?? null,
        country_code: location.country_code ?? null,
        city: location.city ?? null,
        address: location.address ?? null,
        email: contact.email ?? null,
        phone: contact.phone ?? null,
        category_names: categories.map(categoryName).filter(Boolean).join(', '),
        category_slugs: categories.map(categorySlug).filter(Boolean).join(', '),
        social_media: details.social_media ?? null,
        request_locale: context.params.locale ?? null,
        response_source: metadata.source ?? scrapeMetadata.source ?? null,
        scraped_at: metadata.scraped_at ?? scrapeMetadata.scraped_at ?? null,
    };
}

export function buildTrustpilotCompanyDetailsOutputSummary(
    context: TrustpilotCompanyDetailsOutputSummaryContext,
): Record<string, unknown> {
    return {
        request: {
            endpoint: '/trustpilot/company-details',
            company_domains: context.domains,
            ...context.baseParams,
        },
        companies_requested: context.domains.length,
        companies_saved: context.savedCompanies,
        companies_failed: context.failures.length,
        responses_saved: context.savedCompanies,
        status_message: context.statusMessage,
        failures: context.failures,
    };
}
