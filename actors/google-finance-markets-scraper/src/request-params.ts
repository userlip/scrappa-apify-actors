export interface GoogleFinanceMarketsInput {
    trend?: unknown;
    index_market?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export const MARKET_TRENDS = [
    'indexes',
    'most-active',
    'gainers',
    'losers',
    'climate-leaders',
    'cryptocurrencies',
    'currencies',
] as const;

export const INDEX_MARKETS = [
    'americas',
    'europe-middle-east-africa',
    'asia-pacific',
] as const;

const DEFAULT_TREND: typeof MARKET_TRENDS[number] = 'gainers';

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
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

function cleanLanguageCode(value: unknown): string | undefined {
    const hl = cleanString(value, 'hl', 10);
    if (hl === undefined) {
        return undefined;
    }

    const normalized = hl.toLowerCase();
    if (!/^[a-z]{2}(-[a-z]{2})?$/.test(normalized)) {
        throw new Error('hl must be a valid language code such as en, de, or zh-cn');
    }

    return normalized;
}

function cleanCountryCode(value: unknown): string | undefined {
    const gl = cleanString(value, 'gl', 10);
    if (gl === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(gl)) {
        throw new Error('gl must be a two-letter country code');
    }

    return gl.toLowerCase();
}

function cleanTrend(value: unknown): typeof MARKET_TRENDS[number] | undefined {
    const trend = cleanString(value, 'trend', 40);
    if (trend === undefined) {
        return undefined;
    }

    const normalized = trend.toLowerCase();
    if (!MARKET_TRENDS.includes(normalized as typeof MARKET_TRENDS[number])) {
        throw new Error(`trend must be one of: ${MARKET_TRENDS.join(', ')}`);
    }

    return normalized as typeof MARKET_TRENDS[number];
}

function cleanIndexMarket(value: unknown): typeof INDEX_MARKETS[number] | undefined {
    const indexMarket = cleanString(value, 'index_market', 40);
    if (indexMarket === undefined) {
        return undefined;
    }

    const normalized = indexMarket.toLowerCase();
    if (!INDEX_MARKETS.includes(normalized as typeof INDEX_MARKETS[number])) {
        throw new Error(`index_market must be one of: ${INDEX_MARKETS.join(', ')}`);
    }

    return normalized as typeof INDEX_MARKETS[number];
}

export function buildGoogleFinanceMarketsParams(input: GoogleFinanceMarketsInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        trend: DEFAULT_TREND,
    };

    const trend = cleanTrend(input.trend);
    const indexMarket = cleanIndexMarket(input.index_market);
    const hl = cleanLanguageCode(input.hl);
    const gl = cleanCountryCode(input.gl);

    if (trend !== undefined) params.trend = trend;
    if (indexMarket !== undefined) params.index_market = indexMarket;
    if (hl !== undefined) params.hl = hl;
    if (gl !== undefined) params.gl = gl;

    return params;
}

export function describeGoogleFinanceMarketsRequest(params: Record<string, unknown>): string {
    const requestType = typeof params.trend === 'string' ? `trend=${params.trend}` : 'markets overview';
    const filters = ['index_market', 'hl', 'gl']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `${requestType}${filterSuffix}`;
}
