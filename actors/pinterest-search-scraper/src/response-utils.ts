export interface PinterestSearchResponse {
    query?: string;
    count?: number;
    results_count?: number;
    nextBookmark?: string | null;
    bookmark?: string | null;
    pins?: PinterestPin[];
    data?: {
        pins?: PinterestPin[];
        results?: PinterestPin[];
        [key: string]: unknown;
    };
    results?: PinterestPin[];
    [key: string]: unknown;
}

export interface PinterestPin {
    id?: string | number;
    title?: string | null;
    description?: string | null;
    images?: unknown;
    image?: string;
    image_url?: string;
    url?: string;
    link?: string;
    domain?: string;
    pinner?: unknown;
    board?: unknown;
    video?: unknown;
    repin_count?: number;
    comment_count?: number;
    like_count?: number;
    save_count?: number;
    [key: string]: unknown;
}

export type PinterestPinsSource = 'pins' | 'data.pins' | 'results' | 'data.results';

export interface PinterestPinsSelection {
    pins: PinterestPin[];
    source: PinterestPinsSource | null;
}

export function selectPinterestPins(response: PinterestSearchResponse): PinterestPinsSelection {
    if (Array.isArray(response.pins)) {
        return { pins: response.pins, source: 'pins' };
    }

    if (Array.isArray(response.data?.pins)) {
        return { pins: response.data.pins, source: 'data.pins' };
    }

    if (Array.isArray(response.results)) {
        return { pins: response.results, source: 'results' };
    }

    if (Array.isArray(response.data?.results)) {
        return { pins: response.data.results, source: 'data.results' };
    }

    return { pins: [], source: null };
}

export function getPinterestPins(response: PinterestSearchResponse): PinterestPin[] {
    return selectPinterestPins(response).pins;
}

export function limitPinterestSearchResponse(
    response: PinterestSearchResponse | null | undefined,
    limit: number,
    selectedSource?: PinterestPinsSource | null,
): PinterestSearchResponse {
    if (!response) {
        return {};
    }

    const source = selectedSource === undefined
        ? selectPinterestPins(response).source
        : selectedSource;
    const limitedResponse: PinterestSearchResponse = { ...response };

    if (Array.isArray(response.pins) && source === 'pins') {
        limitedResponse.pins = response.pins.slice(0, limit);
    } else {
        delete limitedResponse.pins;
    }

    if (Array.isArray(response.results) && source === 'results') {
        limitedResponse.results = response.results.slice(0, limit);
    } else {
        delete limitedResponse.results;
    }

    if (response.data && typeof response.data === 'object') {
        limitedResponse.data = { ...response.data };

        if (Array.isArray(response.data.pins) && source === 'data.pins') {
            limitedResponse.data.pins = response.data.pins.slice(0, limit);
        } else {
            delete limitedResponse.data.pins;
        }

        if (Array.isArray(response.data.results) && source === 'data.results') {
            limitedResponse.data.results = response.data.results.slice(0, limit);
        } else {
            delete limitedResponse.data.results;
        }
    }

    return limitedResponse;
}

function firstImageUrl(pin: PinterestPin): string | null {
    if (typeof pin.image_url === 'string' && pin.image_url !== '') {
        return pin.image_url;
    }

    if (typeof pin.image === 'string' && pin.image !== '') {
        return pin.image;
    }

    const images = pin.images;
    if (Array.isArray(images)) {
        const firstImage = images.find((image) => {
            return typeof image === 'string'
                || Boolean(image && typeof image === 'object' && typeof (image as Record<string, unknown>).url === 'string');
        });

        if (typeof firstImage === 'string') {
            return firstImage;
        }

        if (firstImage && typeof firstImage === 'object') {
            return ((firstImage as Record<string, unknown>).url as string | undefined) ?? null;
        }
    }

    if (images && typeof images === 'object') {
        const record = images as Record<string, unknown>;
        for (const key of ['orig', 'original', '736x', '564x', '236x']) {
            const value = record[key];
            if (typeof value === 'string' && value !== '') {
                return value;
            }
            if (value && typeof value === 'object' && typeof (value as Record<string, unknown>).url === 'string') {
                return (value as Record<string, unknown>).url as string;
            }
        }
    }

    return null;
}

function objectValue(value: unknown, keys: string[]): unknown {
    if (!value || typeof value !== 'object') {
        return null;
    }

    const record = value as Record<string, unknown>;
    for (const key of keys) {
        if (record[key] !== undefined && record[key] !== null && record[key] !== '') {
            return record[key];
        }
    }

    return null;
}

export function buildPinterestDatasetItem(
    pin: PinterestPin,
    params: Record<string, unknown>,
    response: PinterestSearchResponse,
): Record<string, unknown> {
    return {
        ...pin,
        id: pin.id ?? null,
        title: pin.title ?? null,
        description: pin.description ?? null,
        image_url: firstImageUrl(pin),
        link: pin.link ?? pin.url ?? null,
        domain: pin.domain ?? null,
        pinner_id: objectValue(pin.pinner, ['id', 'user_id']),
        pinner_username: objectValue(pin.pinner, ['username', 'userName', 'name']),
        board_id: objectValue(pin.board, ['id', 'board_id']),
        board_name: objectValue(pin.board, ['name', 'title']),
        has_video: Boolean(pin.video),
        repin_count: pin.repin_count ?? null,
        comment_count: pin.comment_count ?? null,
        like_count: pin.like_count ?? null,
        save_count: pin.save_count ?? null,
        request_query: params.query ?? response.query ?? null,
        request_limit: params.limit ?? null,
        request_bookmark: params.bookmark ?? null,
        count: response.count ?? null,
        results_count: response.results_count ?? getPinterestPins(response).length,
        nextBookmark: response.nextBookmark ?? null,
    };
}
