export interface GoogleTrendsTimelinePoint {
    timestamp?: number;
    date?: string;
    value?: number;
    [key: string]: unknown;
}

export interface GoogleTrendsInterestResponse {
    search_parameters?: Record<string, unknown>;
    timeline_data?: GoogleTrendsTimelinePoint[];
    interest_over_time?: {
        average?: number;
        max_value?: number;
        min_value?: number;
        data_points?: GoogleTrendsTimelinePoint[];
        [key: string]: unknown;
    };
    response_time_ms?: number;
    [key: string]: unknown;
}

function asTimelinePoints(value: unknown): GoogleTrendsTimelinePoint[] {
    return Array.isArray(value) ? value.filter((item): item is GoogleTrendsTimelinePoint => {
        return item !== null && typeof item === 'object' && !Array.isArray(item);
    }) : [];
}

export function getTimelinePoints(response: GoogleTrendsInterestResponse): GoogleTrendsTimelinePoint[] {
    const timelineData = asTimelinePoints(response.timeline_data);
    if (timelineData.length > 0) {
        return timelineData;
    }

    return asTimelinePoints(response.interest_over_time?.data_points);
}

export function buildTimelineDatasetItems(
    response: GoogleTrendsInterestResponse,
    params: Record<string, unknown>,
): Record<string, unknown>[] {
    const timelinePoints = getTimelinePoints(response);
    const interest = response.interest_over_time ?? {};

    return timelinePoints.map((point, index) => ({
        ...point,
        position: index + 1,
        timestamp: point.timestamp ?? null,
        date: point.date ?? null,
        value: point.value ?? null,
        average: interest.average ?? null,
        max_value: interest.max_value ?? null,
        min_value: interest.min_value ?? null,
        request_q: params.q ?? null,
        request_geo: params.geo ?? null,
        request_time_range: params.time_range ?? null,
        request_hl: params.hl ?? null,
        request_search_type: params.search_type ?? null,
        response_time_ms: response.response_time_ms ?? null,
        search_parameters: response.search_parameters ?? null,
    }));
}
