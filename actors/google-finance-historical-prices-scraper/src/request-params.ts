export interface GoogleFinanceHistoricalPricesInput {
    symbol?: unknown;
    exchange?: unknown;
    range?: unknown;
    start_date?: unknown;
    end_date?: unknown;
    interval?: unknown;
    hl?: unknown;
    gl?: unknown;
}

const INTERVAL_VALUES = ['daily', 'weekly', 'monthly'] as const;

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

function cleanRange(value: unknown): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const range = typeof value === 'number' ? value : Number(value);
    if (!Number.isInteger(range)) {
        throw new Error('range must be an integer');
    }

    if (range < 1 || range > 8) {
        throw new Error('range must be between 1 and 8');
    }

    return range;
}

function cleanDate(value: unknown, field: string): string | undefined {
    const date = cleanString(value, field, 10);
    if (date === undefined) {
        return undefined;
    }

    if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
        throw new Error(`${field} must be in YYYY-MM-DD format`);
    }

    const parsed = new Date(`${date}T00:00:00.000Z`);
    if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
        throw new Error(`${field} must be a valid date`);
    }

    const today = new Date();
    const todayUtc = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate()));
    if (parsed.getTime() > todayUtc.getTime()) {
        throw new Error(`${field} cannot be in the future`);
    }

    return date;
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

    return gl.toLowerCase();
}

function cleanInterval(value: unknown): typeof INTERVAL_VALUES[number] | undefined {
    const interval = cleanString(value, 'interval', 20);
    if (interval === undefined) {
        return undefined;
    }

    const normalized = interval.toLowerCase();
    if (!INTERVAL_VALUES.includes(normalized as typeof INTERVAL_VALUES[number])) {
        throw new Error(`interval must be one of: ${INTERVAL_VALUES.join(', ')}`);
    }

    return normalized as typeof INTERVAL_VALUES[number];
}

export function buildGoogleFinanceHistoricalPricesParams(input: GoogleFinanceHistoricalPricesInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        symbol: cleanSymbol(input.symbol),
    };

    const exchange = cleanExchange(input.exchange);
    const range = cleanRange(input.range);
    const startDate = cleanDate(input.start_date, 'start_date');
    const endDate = cleanDate(input.end_date, 'end_date');
    const interval = cleanInterval(input.interval);
    const hl = cleanLanguageCode(input.hl);
    const gl = cleanCountryCode(input.gl);

    if (range !== undefined && (startDate !== undefined || endDate !== undefined)) {
        throw new Error('Cannot use both range and start_date/end_date parameters together. Choose one approach.');
    }

    if ((startDate === undefined) !== (endDate === undefined)) {
        throw new Error('start_date and end_date must be provided together');
    }

    if (startDate !== undefined && endDate !== undefined && startDate > endDate) {
        throw new Error('end_date must be on or after start_date');
    }

    if (exchange !== undefined) params.exchange = exchange;
    if (range !== undefined) params.range = range;
    if (startDate !== undefined) params.start_date = startDate;
    if (endDate !== undefined) params.end_date = endDate;
    if (interval !== undefined) params.interval = interval;
    if (hl !== undefined) params.hl = hl;
    if (gl !== undefined) params.gl = gl;

    return params;
}

export function describeGoogleFinanceHistoricalPricesRequest(params: Record<string, unknown>): string {
    const symbol = typeof params.symbol === 'string' ? params.symbol : 'unknown symbol';
    const exchange = typeof params.exchange === 'string' ? `:${params.exchange}` : '';
    const filters = ['range', 'start_date', 'end_date', 'interval', 'hl', 'gl']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `${symbol}${exchange}${filterSuffix}`;
}
