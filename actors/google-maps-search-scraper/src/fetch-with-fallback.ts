export interface BusinessResult {
    [key: string]: unknown;
}

export interface GoogleMapsSearchResponse {
    items?: BusinessResult[];
    [key: string]: unknown;
}

export interface ScrappaLikeClient {
    get<T>(endpoint: string, params?: Record<string, unknown>): Promise<T>;
}

export function isTransientUpstreamError(message: string): boolean {
    if (/Scrappa API error \((408|429|5\d\d)\)/.test(message)) {
        return true;
    }

    return /timed?\s*out|timeout|temporarily unavailable|cloudflare/i.test(message);
}

export async function fetchWithFallback(
    client: ScrappaLikeClient,
    baseParams: Record<string, unknown>,
    fallbackZoom = 13
): Promise<GoogleMapsSearchResponse> {
    try {
        return await client.get<GoogleMapsSearchResponse>('/maps/simple-search', baseParams);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (!isTransientUpstreamError(message)) {
            throw error;
        }

        console.warn(`Transient upstream issue on simple-search: ${message}`);
    }

    const advancedParams: Record<string, unknown> = {
        ...baseParams,
        zoom: fallbackZoom,
    };

    const advancedResponse = await client.get<GoogleMapsSearchResponse>('/maps/advanced-search', advancedParams);
    return {
        ...advancedResponse,
        fallback_used: 'advanced-search',
    };
}
