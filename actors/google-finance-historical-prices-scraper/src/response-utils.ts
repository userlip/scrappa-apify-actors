export interface GoogleFinancePricePoint {
    date?: number | string;
    close?: number | string;
    change?: number | string;
    percent_change?: number | string;
    volume?: number | string;
    [key: string]: unknown;
}

export interface GoogleFinanceHistoricalPricesResponse {
    symbol?: string | null;
    exchange?: string | null;
    currency?: string | null;
    previous_close?: number | string | null;
    prices?: GoogleFinancePricePoint[];
    [key: string]: unknown;
}

function asPricePoints(value: unknown): GoogleFinancePricePoint[] {
    return Array.isArray(value) ? value.filter((item): item is GoogleFinancePricePoint => {
        return item !== null && typeof item === 'object' && !Array.isArray(item);
    }) : [];
}

function firstString(...values: unknown[]): string | undefined {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return undefined;
}

function asNumber(value: unknown): number | null {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }

    if (typeof value !== 'string') {
        return null;
    }

    const cleaned = value.replace(/,/g, '').trim();
    if (cleaned === '') {
        return null;
    }

    const parsed = Number(cleaned);
    return Number.isFinite(parsed) ? parsed : null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        const parsed = asNumber(value);
        if (parsed !== null) {
            return parsed;
        }
    }

    return null;
}

function timestampToIsoDate(value: unknown): string | null {
    const timestamp = asNumber(value);
    if (timestamp === null) {
        return null;
    }

    const milliseconds = timestamp > 100000000000 ? timestamp : timestamp * 1000;
    const date = new Date(milliseconds);
    return Number.isNaN(date.getTime()) ? null : date.toISOString().slice(0, 10);
}

export function buildHistoricalPriceDatasetItems(
    response: GoogleFinanceHistoricalPricesResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const prices = asPricePoints(response.prices);

    return prices.map((price, index) => ({
        ...price,
        position: index + 1,
        date: firstNumber(price.date),
        date_iso: timestampToIsoDate(price.date),
        close: firstNumber(price.close),
        change: firstNumber(price.change),
        percent_change: firstNumber(price.percent_change),
        volume: firstNumber(price.volume),
        symbol: firstString(response.symbol, params.symbol) ?? null,
        exchange: firstString(response.exchange, params.exchange) ?? null,
        currency: firstString(response.currency) ?? null,
        previous_close: firstNumber(response.previous_close),
        request_symbol: params.symbol ?? null,
        request_exchange: params.exchange ?? null,
        request_range: params.range ?? null,
        request_start_date: params.start_date ?? null,
        request_end_date: params.end_date ?? null,
        request_interval: params.interval ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
        result_counts: {
            prices: prices.length,
        },
    }));
}
