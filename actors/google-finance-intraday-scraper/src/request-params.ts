export interface GoogleFinanceIntradaySymbolInput {
    symbol?: unknown;
    exchange?: unknown;
}

export interface GoogleFinanceIntradayInput {
    symbols?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export interface GoogleFinanceIntradayRequest {
    symbol: string;
    exchange?: string;
    hl?: string;
    gl?: string;
    [key: string]: unknown;
}

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

function cleanSymbol(value: unknown, index: number): string {
    const field = `symbols[${index}].symbol`;
    const symbol = cleanRequiredString(value, field, 20).toUpperCase();
    if (/\s/.test(symbol)) {
        throw new Error(`${field} cannot contain spaces`);
    }

    return symbol;
}

function cleanExchange(value: unknown, index: number): string | undefined {
    const exchange = cleanString(value, `symbols[${index}].exchange`, 40);
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
    const gl = cleanString(value, 'gl', 10);
    if (gl === undefined) {
        return undefined;
    }

    const normalized = gl.toLowerCase();
    if (!/^[a-z]{2}$/.test(normalized)) {
        throw new Error('gl must be a two-letter country code');
    }

    return normalized;
}

function cleanSymbols(value: unknown): GoogleFinanceIntradaySymbolInput[] {
    if (!Array.isArray(value)) {
        throw new Error('symbols must be an array');
    }

    if (value.length === 0) {
        throw new Error('At least one symbol is required');
    }

    return value.map((item, index) => {
        if (item === null || typeof item !== 'object' || Array.isArray(item)) {
            throw new Error(`symbols[${index}] must be an object`);
        }

        return item as GoogleFinanceIntradaySymbolInput;
    });
}

export function buildGoogleFinanceIntradayRequests(input: GoogleFinanceIntradayInput): GoogleFinanceIntradayRequest[] {
    const symbols = cleanSymbols(input.symbols);
    const hl = cleanLanguageCode(input.hl);
    const gl = cleanCountryCode(input.gl);

    return symbols.map((symbolInput, index) => {
        const request: GoogleFinanceIntradayRequest = {
            symbol: cleanSymbol(symbolInput.symbol, index),
        };
        const exchange = cleanExchange(symbolInput.exchange, index);

        if (exchange !== undefined) request.exchange = exchange;
        if (hl !== undefined) request.hl = hl;
        if (gl !== undefined) request.gl = gl;

        return request;
    });
}

export function describeGoogleFinanceIntradayRequest(params: Record<string, unknown>): string {
    const symbol = String(params.symbol ?? '');
    const exchange = params.exchange ? `:${String(params.exchange)}` : '';
    const details = [
        params.hl ? `hl=${params.hl}` : null,
        params.gl ? `gl=${params.gl}` : null,
    ].filter(Boolean);

    return details.length > 0 ? `${symbol}${exchange} (${details.join(', ')})` : `${symbol}${exchange}`;
}
