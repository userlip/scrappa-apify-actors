export interface GoogleVideoResult {
    position?: number;
    title?: string;
    link?: string;
    displayed_link?: string;
    thumbnail?: string;
    snippet?: string;
    duration?: string;
    date?: string;
    video_link?: string;
    source?: string;
    channel?: string;
    rich_snippet?: unknown;
    key_moments?: unknown[];
    [key: string]: unknown;
}

export interface GoogleVideosResponse {
    video_results?: GoogleVideoResult[];
    found_in_videos?: GoogleVideoResult[];
    short_videos?: GoogleVideoResult[];
    related_searches?: unknown[];
    pagination?: unknown;
    scrappa_pagination?: unknown;
    search_information?: unknown;
    search_parameters?: unknown;
    [key: string]: unknown;
}

export function extractVideoResults(response: unknown): GoogleVideoResult[] {
    if (Array.isArray(response)) {
        return response;
    }

    if (response && typeof response === 'object') {
        const payload = response as GoogleVideosResponse;
        if (Array.isArray(payload.video_results)) {
            return payload.video_results;
        }

        if (Array.isArray((payload as { data?: unknown }).data)) {
            return (payload as { data: GoogleVideoResult[] }).data;
        }
    }

    console.warn('Scrappa Google Videos response did not include a video result array');
    return [];
}

export function enrichResult(result: GoogleVideoResult, params: Record<string, unknown>): Record<string, unknown> {
    // Scrappa's Google Videos payload uses link for the destination URL and video_link for Google's redirect URL.
    return {
        ...result,
        position: result.position ?? null,
        title: result.title ?? null,
        video_url: result.link ?? result.video_link ?? null,
        google_redirect_url: result.video_link ?? null,
        source_url: result.link ?? null,
        displayed_link: result.displayed_link ?? null,
        thumbnail_url: result.thumbnail ?? null,
        snippet: result.snippet ?? null,
        duration: result.duration ?? null,
        date: result.date ?? null,
        key_moments_count: Array.isArray(result.key_moments) ? result.key_moments.length : 0,
        request_q: params.q ?? null,
        request_page: params.page ?? null,
        request_start: params.start ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
        request_google_domain: params.google_domain ?? null,
        request_location: params.location ?? null,
        request_uule: params.uule ?? null,
        request_tbs: params.tbs ?? null,
        request_safe: params.safe ?? null,
        request_filter: params.filter ?? null,
        request_nfpr: params.nfpr ?? null,
        request_lr: params.lr ?? null,
    };
}
