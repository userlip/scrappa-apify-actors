export interface SimilarwebTrafficResponse {
    domain?: string | null;
    site_name?: string | null;
    title?: string | null;
    description?: string | null;
    category?: string | null;
    global_rank?: unknown;
    country_rank?: unknown;
    country_code?: string | number | null;
    category_rank?: unknown;
    engagement?: Record<string, unknown>;
    traffic_sources?: Record<string, unknown>;
    top_countries?: unknown[];
    estimated_monthly_visits?: Record<string, unknown>;
    monthly_visits?: Record<string, unknown>;
    top_keywords?: unknown[];
    screenshot?: string | null;
    [key: string]: unknown;
}

function asRecord(value: unknown): Record<string, unknown> {
    return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function asArray(value: unknown): unknown[] {
    return Array.isArray(value) ? value : [];
}

function firstString(...values: unknown[]): string | undefined {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return undefined;
}

function firstNumber(...values: unknown[]): number | undefined {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }

        if (typeof value === 'string') {
            const cleaned = value.replace(/,/g, '').trim();
            if (cleaned === '') {
                continue;
            }

            const parsed = Number(cleaned);
            if (Number.isFinite(parsed)) {
                return parsed;
            }
        }
    }

    return undefined;
}

function rankValue(value: unknown): number | string | null {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }

    if (typeof value === 'string' && value.trim() !== '') {
        const parsed = Number(value.replace(/,/g, '').trim());
        return Number.isFinite(parsed) ? parsed : value.trim();
    }

    const record = asRecord(value);
    const rank = firstNumber(record.Rank, record.rank);
    if (rank !== undefined) {
        return rank;
    }

    return firstString(record.Rank, record.rank) ?? null;
}

function latestEstimatedVisitMonth(visits: Record<string, unknown>): string | null {
    const months = Object.keys(visits).sort();
    return months.length > 0 ? months[months.length - 1] : null;
}

function hasVisitValues(visits: Record<string, unknown>): boolean {
    return Object.values(visits).some((value) => firstNumber(value) !== undefined);
}

export function hasSimilarwebTrafficData(response: SimilarwebTrafficResponse): boolean {
    return rankValue(response.global_rank) !== null
        || firstNumber(asRecord(response.engagement).visits) !== undefined
        || hasVisitValues(asRecord(response.estimated_monthly_visits))
        || hasVisitValues(asRecord(response.monthly_visits));
}

export function buildSimilarwebDatasetItem(
    response: SimilarwebTrafficResponse,
    request: { domain: string; inputDomain: string },
): Record<string, unknown> {
    const engagement = asRecord(response.engagement);
    const trafficSources = asRecord(response.traffic_sources);
    const monthlyVisits = asRecord(response.monthly_visits);
    const estimatedMonthlyVisits = {
        ...monthlyVisits,
        ...asRecord(response.estimated_monthly_visits),
    };
    const latestMonth = latestEstimatedVisitMonth(estimatedMonthlyVisits);

    return {
        success: true,
        domain: firstString(response.domain, request.domain) ?? request.domain,
        site_name: response.site_name ?? null,
        title: response.title ?? null,
        description: response.description ?? null,
        category: response.category ?? null,
        global_rank_value: rankValue(response.global_rank),
        global_rank: response.global_rank ?? null,
        country_rank_value: rankValue(response.country_rank),
        country_rank: response.country_rank ?? null,
        country_code: response.country_code ?? firstString(asRecord(response.country_rank).CountryCode) ?? null,
        category_rank_value: rankValue(response.category_rank),
        category_rank: response.category_rank ?? null,
        visits: firstNumber(engagement.visits) ?? null,
        time_on_site: firstNumber(engagement.time_on_site) ?? null,
        page_per_visit: firstNumber(engagement.page_per_visit) ?? null,
        bounce_rate: firstNumber(engagement.bounce_rate) ?? null,
        engagement_month: firstNumber(engagement.month) ?? null,
        engagement_year: firstNumber(engagement.year) ?? null,
        traffic_direct: firstNumber(trafficSources.direct) ?? null,
        traffic_search: firstNumber(trafficSources.search) ?? null,
        traffic_social: firstNumber(trafficSources.social) ?? null,
        traffic_referrals: firstNumber(trafficSources.referrals) ?? null,
        traffic_mail: firstNumber(trafficSources.mail) ?? null,
        traffic_paid_referrals: firstNumber(trafficSources.paid_referrals) ?? null,
        top_countries: asArray(response.top_countries),
        top_keywords: asArray(response.top_keywords),
        monthly_visits: monthlyVisits,
        estimated_monthly_visits: estimatedMonthlyVisits,
        latest_month: latestMonth,
        latest_month_visits: latestMonth === null ? null : firstNumber(estimatedMonthlyVisits[latestMonth]) ?? null,
        screenshot: response.screenshot ?? null,
        request_domain: request.domain,
        input_domain: request.inputDomain,
        result_counts: {
            top_countries: asArray(response.top_countries).length,
            top_keywords: asArray(response.top_keywords).length,
            monthly_visit_months: Object.keys(monthlyVisits).length,
            estimated_monthly_visit_months: Object.keys(estimatedMonthlyVisits).length,
        },
    };
}
