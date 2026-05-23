export type GoogleFinanceSearchResponse = Record<string, unknown>;

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

function buildGoogleFinanceUrl(record: Record<string, unknown>): string | null {
    const link = firstString(record.link, record.url, record.google_finance_url);
    if (link !== undefined) {
        return link;
    }

    const stock = firstString(record.stock);
    if (stock !== undefined) {
        return `https://www.google.com/finance/quote/${encodeURIComponent(stock)}`;
    }

    const symbol = firstString(record.symbol);
    const exchange = firstString(record.exchange);
    if (symbol !== undefined && exchange !== undefined) {
        return `https://www.google.com/finance/quote/${encodeURIComponent(`${symbol}:${exchange}`)}`;
    }

    return null;
}

function extractSearchResults(response: GoogleFinanceSearchResponse): unknown[] {
    for (const key of ['results', 'search_results', 'items']) {
        const results = asArray(response[key]);
        if (results.length > 0) {
            return results;
        }
    }

    const data = asRecord(response.data);
    for (const key of ['results', 'search_results', 'items']) {
        const results = asArray(data[key]);
        if (results.length > 0) {
            return results;
        }
    }

    return [];
}

export function buildSearchDatasetItems(
    response: GoogleFinanceSearchResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const query = typeof params.q === 'string' ? params.q : null;

    return extractSearchResults(response).map((result, index) => {
        const record = asRecord(result);
        const priceMovement = asRecord(record.price_movement);
        const googleFinanceUrl = buildGoogleFinanceUrl(record);

        return {
            query,
            position: index + 1,
            name: firstString(record.name, record.title) ?? null,
            symbol: firstString(record.symbol) ?? null,
            exchange: firstString(record.exchange) ?? null,
            stock: firstString(record.stock) ?? null,
            type: firstString(record.type, record.instrument_type, record.asset_type) ?? null,
            currency: firstString(record.currency) ?? null,
            price: firstNumber(record.price, record.extracted_price, record.current_price) ?? null,
            price_change: firstNumber(record.price_change, record.change, priceMovement.value) ?? null,
            percent_change: firstNumber(record.percent_change, record.change_percent, priceMovement.percentage) ?? null,
            link: firstString(record.link, record.url) ?? googleFinanceUrl,
            google_finance_url: googleFinanceUrl,
            market: firstString(record.market, record.region) ?? null,
            request_hl: params.hl ?? null,
            request_gl: params.gl ?? null,
            raw_result: record,
        };
    });
}

export function countSearchResults(response: GoogleFinanceSearchResponse): number {
    return extractSearchResults(response).length;
}
