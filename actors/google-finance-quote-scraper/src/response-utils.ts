export interface GoogleFinanceQuoteResponse {
    quote?: {
        summary?: Record<string, unknown>;
        key_stats?: Record<string, unknown>;
        about?: Record<string, unknown>;
        financials?: unknown[];
        news?: unknown[];
        discover_more?: unknown[];
        [key: string]: unknown;
    };
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

function hasMeaningfulValue(value: unknown): boolean {
    if (typeof value === 'string') {
        return value.trim() !== '';
    }

    if (typeof value === 'number') {
        return Number.isFinite(value);
    }

    if (typeof value === 'boolean') {
        return true;
    }

    if (Array.isArray(value)) {
        return value.some(hasMeaningfulValue);
    }

    if (value && typeof value === 'object') {
        return Object.values(value as Record<string, unknown>).some(hasMeaningfulValue);
    }

    return false;
}

export function hasMeaningfulQuoteData(response: GoogleFinanceQuoteResponse): boolean {
    const quote = asRecord(response.quote);
    const summary = asRecord(quote.summary);
    const about = asRecord(quote.about);
    const keyStats = asRecord(quote.key_stats);
    const financials = asArray(quote.financials);
    const news = asArray(quote.news);
    const discoverMore = asArray(quote.discover_more);

    if (firstNumber(summary.current_price, summary.price, summary.last_price) !== undefined) {
        return true;
    }

    if (firstNumber(summary.price_change, summary.change, summary.percent_change, summary.change_percent) !== undefined) {
        return true;
    }

    if (firstString(summary.market_status, summary.market_state, summary.market, about.description) !== undefined) {
        return true;
    }

    if (financials.length > 0 || news.length > 0 || discoverMore.length > 0) {
        return true;
    }

    return Object.values(keyStats).some(hasMeaningfulValue);
}

function firstArrayString(value: unknown): string | undefined {
    if (!Array.isArray(value)) {
        return undefined;
    }

    return firstString(...value);
}

function extractRelatedTickers(discoverMore: unknown[]): unknown[] {
    const related: unknown[] = [];

    for (const section of discoverMore) {
        if (!section || typeof section !== 'object') {
            continue;
        }

        const sectionRecord = section as Record<string, unknown>;
        for (const key of ['items', 'results', 'tickers', 'quotes']) {
            const items = sectionRecord[key];
            if (Array.isArray(items)) {
                related.push(...items);
            }
        }
    }

    return related;
}

export function buildQuoteDatasetItem(
    response: GoogleFinanceQuoteResponse,
    params: Record<string, unknown>,
): Record<string, unknown> {
    const quote = asRecord(response.quote);
    const summary = asRecord(quote.summary);
    const keyStats = asRecord(quote.key_stats);
    const about = asRecord(quote.about);
    const financials = asArray(quote.financials);
    const news = asArray(quote.news);
    const discoverMore = asArray(quote.discover_more);
    const relatedTickers = extractRelatedTickers(discoverMore);

    return {
        ...summary,
        symbol: firstString(summary.symbol, params.symbol) ?? null,
        exchange: firstString(summary.exchange, params.exchange) ?? null,
        name: firstString(summary.name, summary.title, about.name) ?? null,
        current_price: firstNumber(summary.current_price, summary.price, summary.last_price) ?? null,
        currency: firstString(summary.currency, keyStats.currency, firstArrayString(summary.extensions)) ?? null,
        price_change: firstNumber(summary.price_change, summary.change) ?? null,
        percent_change: firstNumber(summary.percent_change, summary.change_percent) ?? null,
        market_status: firstString(summary.market_status, summary.market_state, summary.market) ?? null,
        key_stats: keyStats,
        about,
        financials,
        news,
        discover_more: discoverMore,
        related_tickers: relatedTickers,
        request_symbol: params.symbol ?? null,
        request_exchange: params.exchange ?? null,
        request_period_type: params.period_type ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
        result_counts: {
            financials: financials.length,
            news: news.length,
            discover_more: discoverMore.length,
            related_tickers: relatedTickers.length,
        },
    };
}
