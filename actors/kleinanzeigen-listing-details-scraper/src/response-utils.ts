export interface KleinanzeigenDetail {
    id?: string | number | null;
    title?: string | null;
    price?: string | number | null;
    price_numeric?: number | null;
    description?: string | null;
    location?: string | null;
    images?: unknown[] | null;
    seller?: Record<string, unknown> | null;
    attributes?: Record<string, unknown> | null;
    shipping?: Record<string, unknown> | null;
    posted_at?: string | null;
    categories?: unknown[] | null;
    [key: string]: unknown;
}

export interface KleinanzeigenDetailsResponse {
    data?: KleinanzeigenDetail | { listing?: KleinanzeigenDetail; result?: KleinanzeigenDetail; item?: KleinanzeigenDetail };
    listing?: KleinanzeigenDetail;
    result?: KleinanzeigenDetail;
    item?: KleinanzeigenDetail;
    [key: string]: unknown;
}

function isObject(value: unknown): value is Record<string, unknown> {
    return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function hasListingId(value: unknown): value is KleinanzeigenDetail {
    if (!isObject(value)) return false;
    const id = value.id;
    return (typeof id === 'string' && id.trim().length > 0)
        || (typeof id === 'number' && Number.isFinite(id));
}

export function selectKleinanzeigenDetail(response: KleinanzeigenDetailsResponse | null | undefined): KleinanzeigenDetail | null {
    const data = response?.data;
    const candidates = [
        isObject(data) ? data.listing : undefined,
        isObject(data) ? data.result : undefined,
        isObject(data) ? data.item : undefined,
        data,
        response?.listing,
        response?.result,
        response?.item,
    ];
    return candidates.find(hasListingId) ?? null;
}

export function buildKleinanzeigenDetailsDatasetItem(
    detail: KleinanzeigenDetail,
    requestAdId: string,
    requestIndex: number,
): Record<string, unknown> {
    return {
        id: detail.id ?? requestAdId,
        title: detail.title ?? null,
        price: detail.price ?? null,
        price_numeric: detail.price_numeric ?? null,
        description: detail.description ?? null,
        location: detail.location ?? null,
        images: Array.isArray(detail.images) ? detail.images : null,
        seller: isObject(detail.seller) ? detail.seller : null,
        attributes: isObject(detail.attributes) ? detail.attributes : null,
        shipping: isObject(detail.shipping) ? detail.shipping : null,
        posted_at: detail.posted_at ?? null,
        categories: Array.isArray(detail.categories) ? detail.categories : null,
        request_ad_id: requestAdId,
        request_index: requestIndex,
    };
}
