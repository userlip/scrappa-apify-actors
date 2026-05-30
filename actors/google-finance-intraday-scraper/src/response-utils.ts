export interface GoogleFinanceIntradayPoint {
    price?: number | string | null;
    change?: number | string | null;
    percent_change?: number | string | null;
    currency?: string | null;
    date?: string | null;
    volume?: number | string | null;
    [key: string]: unknown;
}

export interface GoogleFinanceIntradayResponse {
    symbol?: string | null;
    exchange?: string | null;
    currency?: string | null;
    graph?: GoogleFinanceIntradayPoint[];
    [key: string]: unknown;
}

function asGraphPoints(value: unknown): GoogleFinanceIntradayPoint[] {
    return Array.isArray(value) ? value.filter((item): item is GoogleFinanceIntradayPoint => {
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

function dateToIso(value: unknown): string | null {
    if (typeof value !== 'string' || value.trim() === '') {
        return null;
    }

    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) {
        console.warn(`Could not parse Google Finance intraday date: ${value}`);
        return null;
    }

    return parsed.toISOString();
}

export function buildIntradayPricePointDatasetItems(
    response: GoogleFinanceIntradayResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const graph = asGraphPoints(response.graph);

    return graph.map((point, index) => ({
        ...point,
        position: index + 1,
        date: firstString(point.date) ?? null,
        date_iso: dateToIso(point.date),
        price: firstNumber(point.price),
        change: firstNumber(point.change),
        percent_change: firstNumber(point.percent_change),
        volume: firstNumber(point.volume),
        symbol: firstString(response.symbol, params.symbol) ?? null,
        exchange: firstString(response.exchange, params.exchange) ?? null,
        currency: firstString(point.currency, response.currency) ?? null,
        request_symbol: params.symbol ?? null,
        request_exchange: params.exchange ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
    }));
}
