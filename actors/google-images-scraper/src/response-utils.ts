export interface GoogleImageResult {
    position?: number;
    thumbnail?: string;
    source?: string;
    title?: string;
    link?: string;
    original?: string;
    original_width?: number;
    original_height?: number;
    is_product?: boolean;
    [key: string]: unknown;
}

export type GoogleImagesResponse = GoogleImageResult[] | { data?: GoogleImageResult[]; [key: string]: unknown };

export function extractImageResults(response: unknown): GoogleImageResult[] {
    if (Array.isArray(response)) {
        return response;
    }

    if (response && typeof response === 'object' && Array.isArray((response as { data?: unknown }).data)) {
        const { data } = response as { data: GoogleImageResult[] };
        return data;
    }

    console.warn('Scrappa Google Images response did not include an image result array');
    return [];
}

export function enrichResult(result: GoogleImageResult, params: Record<string, unknown>): Record<string, unknown> {
    return {
        ...result,
        position: result.position ?? null,
        source: result.source ?? null,
        image_url: result.original ?? null,
        thumbnail_url: result.thumbnail ?? null,
        source_url: result.link ?? null,
        width: result.original_width ?? null,
        height: result.original_height ?? null,
        is_product: result.is_product ?? false,
        request_q: params.q ?? null,
        request_page: params.page ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
        request_imgsz: params.imgsz ?? null,
        request_imgtype: params.imgtype ?? null,
        request_imgcolor: params.imgcolor ?? null,
        request_imgar: params.imgar ?? null,
        request_tbs: params.tbs ?? null,
        request_safe: params.safe ?? null,
    };
}
