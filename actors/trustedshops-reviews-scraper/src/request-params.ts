export interface TrustedShopsReviewsInput {
    tsids?: unknown;
    urls?: unknown;
    tsid?: unknown;
    url?: unknown;
    page?: unknown;
    max_pages?: unknown;
    size?: unknown;
    market?: unknown;
    include_raw_responses?: unknown;
}

export interface TrustedShopsTarget {
    tsid: string;
    input: string;
    sourceUrl: string;
}

export interface TrustedShopsReviewsRequestPlan {
    targets: TrustedShopsTarget[];
    baseParams: Record<string, unknown>;
    startPage: number;
    maxPages: number;
    includeRawResponses: boolean;
}

const VALID_MARKETS = ['DEU', 'GBR', 'AUT', 'CHE', 'NLD', 'ESP', 'ITA', 'FRA', 'BEL', 'POL', 'PRT'] as const;
const TSID_PATTERN = /^[A-Z0-9]{33}$/;
const DEFAULT_SIZE = 20;
const MAX_SIZE = 100;
const MAX_TARGETS_PER_RUN = 50;
const MAX_PAGE = 100;
const MAX_PAGES_PER_TARGET = 25;

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'boolean') {
        throw new Error(`${field} must be a boolean`);
    }

    return value;
}

function cleanMarket(value: unknown): string | undefined {
    const market = cleanOptionalString(value, 'market', 3);
    if (market === undefined) {
        return undefined;
    }

    const normalized = market.toUpperCase();
    if (!VALID_MARKETS.includes(normalized as typeof VALID_MARKETS[number])) {
        throw new Error(`market must be one of: ${VALID_MARKETS.join(', ')}`);
    }

    return normalized;
}

function cleanStringArray(value: unknown, field: string): string[] {
    if (value === undefined || value === null || value === '') {
        return [];
    }

    const values = Array.isArray(value) ? value : [value];
    return values
        .map((item) => cleanOptionalString(item, field, 500))
        .filter((item): item is string => item !== undefined);
}

export function extractTrustedShopsTsid(value: string): string | undefined {
    const trimmed = value.trim();
    const directTsid = trimmed.toUpperCase();
    if (TSID_PATTERN.test(directTsid)) {
        return directTsid;
    }

    let decoded = trimmed;
    try {
        decoded = decodeURIComponent(trimmed);
    } catch {
        decoded = trimmed;
    }

    const match = decoded.match(/(?:info_|tsid[=/]|trustedshops\/reviews\/)([A-Z0-9]{33})/i)
        ?? decoded.match(/([A-Z0-9]{33})/i);

    return match?.[1]?.toUpperCase();
}

function normalizeSourceUrl(input: string, tsid: string): string {
    const trimmed = input.trim();
    if (/^https?:\/\//i.test(trimmed)) {
        return trimmed;
    }

    if (/trustedshops\./i.test(trimmed)) {
        return `https://${trimmed.replace(/^\/+/, '')}`;
    }

    return `https://www.trustedshops.de/bewertung/info_${tsid}.html`;
}

function buildTarget(input: string, field: string): TrustedShopsTarget {
    const tsid = extractTrustedShopsTsid(input);
    if (!tsid) {
        throw new Error(`${field} must contain a 33-character Trusted Shops TSID, for example XFB15FFBDE1DEE7A55D292A7D48598A6A`);
    }

    return {
        tsid,
        input,
        sourceUrl: normalizeSourceUrl(input, tsid),
    };
}

function uniqueTargets(targets: TrustedShopsTarget[]): TrustedShopsTarget[] {
    const seen = new Set<string>();
    const unique: TrustedShopsTarget[] = [];

    for (const target of targets) {
        if (seen.has(target.tsid)) {
            continue;
        }

        seen.add(target.tsid);
        unique.push(target);
    }

    return unique;
}

export function buildTrustedShopsReviewsPlan(input: TrustedShopsReviewsInput): TrustedShopsReviewsRequestPlan {
    const targets = uniqueTargets([
        ...cleanStringArray(input.tsids, 'tsids').map((value) => buildTarget(value, 'tsids')),
        ...cleanStringArray(input.urls, 'urls').map((value) => buildTarget(value, 'urls')),
        ...cleanStringArray(input.tsid, 'tsid').map((value) => buildTarget(value, 'tsid')),
        ...cleanStringArray(input.url, 'url').map((value) => buildTarget(value, 'url')),
    ]);

    if (targets.length === 0) {
        throw new Error('Provide at least one Trusted Shops TSID or profile URL in tsids or urls');
    }

    if (targets.length > MAX_TARGETS_PER_RUN) {
        throw new Error(`Provide ${MAX_TARGETS_PER_RUN} or fewer Trusted Shops targets per run`);
    }

    const startPage = cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_TARGET) ?? 1;
    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 100');
    }

    const baseParams: Record<string, unknown> = {
        size: cleanInteger(input.size, 'size', 1, MAX_SIZE) ?? DEFAULT_SIZE,
    };

    const market = cleanMarket(input.market);
    if (market !== undefined) {
        baseParams.market = market;
    }

    return {
        targets,
        baseParams,
        startPage,
        maxPages,
        includeRawResponses: cleanBoolean(input.include_raw_responses, 'include_raw_responses') ?? false,
    };
}

export function buildPageParams(plan: TrustedShopsReviewsRequestPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeTrustedShopsReviewsRequest(plan: TrustedShopsReviewsRequestPlan): string {
    const lastPage = plan.startPage + plan.maxPages - 1;
    const pages = plan.maxPages === 1 ? `page ${plan.startPage}` : `pages ${plan.startPage}-${lastPage}`;
    return `${plan.targets.length} Trusted Shops target(s) (${pages})`;
}
