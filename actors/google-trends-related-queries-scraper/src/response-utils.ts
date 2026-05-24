export type RelatedQueryType = 'top' | 'rising' | null;
export type RelatedResultKind = 'query' | 'topic';

export interface GoogleTrendsRelatedEntry {
    query?: unknown;
    topic?: unknown;
    title?: unknown;
    value?: unknown;
    formatted_value?: unknown;
    link?: unknown;
    type?: unknown;
    [key: string]: unknown;
}

export interface GoogleTrendsRelatedResponse {
    search_parameters?: Record<string, unknown>;
    related_queries?: unknown;
    related_topics?: unknown;
    response_time_ms?: number;
    [key: string]: unknown;
}

interface RelatedGroup {
    type: RelatedQueryType;
    entries: GoogleTrendsRelatedEntry[];
}

function asEntries(value: unknown): GoogleTrendsRelatedEntry[] {
    return Array.isArray(value)
        ? value.filter((item): item is GoogleTrendsRelatedEntry => {
            return item !== null && typeof item === 'object' && !Array.isArray(item);
        })
        : [];
}

function getRelatedGroups(value: unknown): RelatedGroup[] {
    if (Array.isArray(value)) {
        return [{ type: null, entries: asEntries(value) }];
    }

    if (value !== null && typeof value === 'object') {
        const record = value as Record<string, unknown>;
        const groups: RelatedGroup[] = [
            { type: 'top', entries: asEntries(record.top) },
            { type: 'rising', entries: asEntries(record.rising) },
        ];

        return groups.filter((group) => group.entries.length > 0);
    }

    return [];
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return null;
}

function buildDatasetItem(
    entry: GoogleTrendsRelatedEntry,
    params: Record<string, unknown>,
    response: GoogleTrendsRelatedResponse,
    kind: RelatedResultKind,
    type: RelatedQueryType,
    position: number,
): Record<string, unknown> {
    const query = kind === 'query' ? firstString(entry.query, entry.title) : null;
    const topic = kind === 'topic' ? firstString(entry.topic, entry.title, entry.query) : null;
    const topicType = kind === 'topic' ? firstString(entry.type) : null;

    return {
        ...entry,
        position,
        result_kind: kind,
        type,
        query,
        topic,
        topic_type: topicType,
        value: entry.value ?? null,
        formatted_value: entry.formatted_value ?? null,
        link: entry.link ?? null,
        source_keyword: params.q ?? null,
        request_geo: params.geo ?? null,
        request_time_range: params.time_range ?? null,
        request_hl: params.hl ?? null,
        request_search_type: params.search_type ?? null,
        response_time_ms: response.response_time_ms ?? null,
        search_parameters: response.search_parameters ?? null,
    };
}

export function buildRelatedDatasetItems(
    response: GoogleTrendsRelatedResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const items: Record<string, unknown>[] = [];

    for (const [kind, value] of [
        ['query', response.related_queries],
        ['topic', response.related_topics],
    ] as const) {
        const groups = getRelatedGroups(value);

        for (const group of groups) {
            for (const [index, entry] of group.entries.entries()) {
                items.push(buildDatasetItem(entry, params, response, kind, group.type, index + 1));
            }
        }
    }

    return items;
}
