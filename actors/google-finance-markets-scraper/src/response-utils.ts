export interface GoogleFinanceMarketsResponse {
    markets?: Record<string, unknown[]>;
    market_trends?: Array<{
        title?: string | null;
        results?: unknown[];
    }>;
    news_results?: Array<Record<string, unknown>>;
    [key: string]: unknown;
}

const OVERVIEW_SECTION_KEYS = ['us', 'europe', 'asia', 'currencies', 'crypto', 'futures'] as const;

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

function extractPriceMovement(item: Record<string, unknown>): Record<string, unknown> {
    return asRecord(item.price_movement);
}

function buildMarketRow(
    item: unknown,
    section: string,
    position: number,
    params: Record<string, unknown>,
    trendGroupTitle?: string | null,
): Record<string, unknown> {
    const record = asRecord(item);
    const movement = extractPriceMovement(record);

    return {
        item_type: 'market_row',
        section,
        trend: params.trend ?? null,
        trend_group: trendGroupTitle ?? null,
        position,
        stock: firstString(record.stock) ?? null,
        link: firstString(record.link) ?? null,
        name: firstString(record.name) ?? null,
        symbol: firstString(record.symbol) ?? null,
        exchange: firstString(record.exchange) ?? null,
        price: firstNumber(record.price, record.extracted_price) ?? null,
        currency: firstString(record.currency) ?? null,
        price_movement_direction: firstString(movement.direction) ?? null,
        price_movement_value: firstNumber(movement.value) ?? null,
        price_movement_percentage: firstNumber(movement.percentage) ?? null,
        from_currency: firstString(record.from_currency) ?? null,
        to_currency: firstString(record.to_currency) ?? null,
        request_trend: params.trend ?? null,
        request_index_market: params.index_market ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
    };
}

function buildNewsRow(item: unknown, position: number, params: Record<string, unknown>): Record<string, unknown> {
    const record = asRecord(item);

    return {
        item_type: 'news_result',
        section: 'finance-news',
        trend: params.trend ?? null,
        trend_group: null,
        position,
        title: firstString(record.title) ?? null,
        link: firstString(record.link) ?? null,
        source: firstString(record.source) ?? null,
        date: firstString(record.date) ?? null,
        snippet: firstString(record.snippet) ?? null,
        thumbnail: firstString(record.thumbnail) ?? null,
        request_trend: params.trend ?? null,
        request_index_market: params.index_market ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
    };
}

export function buildMarketsDatasetItems(
    response: GoogleFinanceMarketsResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const items: Record<string, unknown>[] = [];

    if (Array.isArray(response.market_trends)) {
        response.market_trends.forEach((group, groupIndex) => {
            const groupRecord = asRecord(group);
            const title = firstString(groupRecord.title) ?? null;
            const section = title ?? (typeof params.trend === 'string' ? params.trend : `trend-${groupIndex + 1}`);
            const results = asArray(groupRecord.results);

            results.forEach((result, index) => {
                items.push(buildMarketRow(result, section, index + 1, params, title));
            });
        });
    }

    const markets = asRecord(response.markets);
    for (const section of OVERVIEW_SECTION_KEYS) {
        const sectionItems = asArray(markets[section]);
        sectionItems.forEach((item, index) => {
            items.push(buildMarketRow(item, section, index + 1, params));
        });
    }

    const newsResults = asArray(response.news_results);
    newsResults.forEach((item, index) => {
        items.push(buildNewsRow(item, index + 1, params));
    });

    return items;
}

export function buildMarketsResultCounts(response: GoogleFinanceMarketsResponse): Record<string, number> {
    const markets = asRecord(response.markets);
    const counts: Record<string, number> = {
        market_rows: 0,
        trend_groups: asArray(response.market_trends).length,
        trend_rows: 0,
        news_results: asArray(response.news_results).length,
    };

    for (const section of OVERVIEW_SECTION_KEYS) {
        const count = asArray(markets[section]).length;
        counts[section] = count;
        counts.market_rows += count;
    }

    for (const group of asArray(response.market_trends)) {
        counts.trend_rows += asArray(asRecord(group).results).length;
    }

    counts.market_rows += counts.trend_rows;

    return counts;
}
