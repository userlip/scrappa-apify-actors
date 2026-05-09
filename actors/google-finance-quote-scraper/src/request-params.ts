export interface GoogleFinanceQuoteInput {
    symbol?: unknown;
    exchange?: unknown;
    period_type?: unknown;
    hl?: unknown;
    gl?: unknown;
}

const PERIOD_TYPE_VALUES = ['quarterly', 'annual'] as const;

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

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function cleanSymbol(value: unknown): string {
    const symbol = cleanRequiredString(value, 'symbol', 20).toUpperCase();
    if (/\s/.test(symbol)) {
        throw new Error('symbol cannot contain spaces');
    }

    return symbol;
}

function cleanExchange(value: unknown): string | undefined {
    const exchange = cleanString(value, 'exchange', 40);
    return exchange === undefined ? undefined : exchange.toUpperCase();
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
    const gl = cleanString(value, 'gl', 2);
    if (gl === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(gl)) {
        throw new Error('gl must be a two-letter country code');
    }

    return gl.toLowerCase();
}

function cleanPeriodType(value: unknown): typeof PERIOD_TYPE_VALUES[number] | undefined {
    const periodType = cleanString(value, 'period_type', 20);
    if (periodType === undefined) {
        return undefined;
    }

    const normalized = periodType.toLowerCase();
    if (!PERIOD_TYPE_VALUES.includes(normalized as typeof PERIOD_TYPE_VALUES[number])) {
        throw new Error(`period_type must be one of: ${PERIOD_TYPE_VALUES.join(', ')}`);
    }

    return normalized as typeof PERIOD_TYPE_VALUES[number];
}

export function buildGoogleFinanceQuoteParams(input: GoogleFinanceQuoteInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        symbol: cleanSymbol(input.symbol),
    };

    const exchange = cleanExchange(input.exchange);
    const periodType = cleanPeriodType(input.period_type);
    const hl = cleanLanguageCode(input.hl);
    const gl = cleanCountryCode(input.gl);

    if (exchange !== undefined) params.exchange = exchange;
    if (periodType !== undefined) params.period_type = periodType;
    if (hl !== undefined) params.hl = hl;
    if (gl !== undefined) params.gl = gl;

    return params;
}

export function describeGoogleFinanceQuoteRequest(params: Record<string, unknown>): string {
    const symbol = typeof params.symbol === 'string' ? params.symbol : 'unknown symbol';
    const exchange = typeof params.exchange === 'string' ? `:${params.exchange}` : '';
    const filters = ['period_type', 'hl', 'gl']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `${symbol}${exchange}${filterSuffix}`;
}
