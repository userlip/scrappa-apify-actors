export interface VintedSearchResponse {
    success?: boolean;
    data?: {
        items?: VintedItem[];
        pagination?: VintedPagination;
        [key: string]: unknown;
    };
    items?: VintedItem[];
    pagination?: VintedPagination;
    meta?: Record<string, unknown>;
    message?: string;
    [key: string]: unknown;
}

export interface VintedPagination {
    current_page?: number;
    page?: number;
    total_pages?: number;
    total_entries?: number;
    total_items?: number;
    per_page?: number;
    has_next_page?: boolean;
    next_page?: number | null;
    [key: string]: unknown;
}

export interface VintedItem {
    id?: string | number;
    title?: string;
    description?: string;
    price?: {
        amount?: number | string;
        currency?: string;
        currency_code?: string;
        [key: string]: unknown;
    } | number | string;
    total_item_price?: { amount?: number | string; currency?: string; currency_code?: string; [key: string]: unknown } | number | string;
    shipping_price?: { amount?: number | string; currency?: string; currency_code?: string; [key: string]: unknown } | number | string;
    service_fee?: { amount?: number | string; currency?: string; currency_code?: string; [key: string]: unknown } | number | string;
    brand?: string | { title?: string; name?: string; [key: string]: unknown };
    brand_title?: string;
    category?: string | { title?: string; name?: string; [key: string]: unknown };
    size?: string | { title?: string; name?: string; [key: string]: unknown };
    size_title?: string;
    color?: string | { title?: string; name?: string; [key: string]: unknown };
    color_title?: string;
    condition?: string;
    condition_title?: string;
    status?: string;
    status_title?: string;
    url?: string;
    path?: string;
    image_url?: string;
    photo_url?: string;
    photo?: { url?: string; [key: string]: unknown };
    photos?: Array<{ url?: string; [key: string]: unknown }>;
    seller?: {
        id?: string | number;
        login?: string;
        username?: string;
        feedback_count?: number;
        feedback_reputation?: number;
        [key: string]: unknown;
    };
    user?: {
        id?: string | number;
        login?: string;
        username?: string;
        feedback_count?: number;
        feedback_reputation?: number;
        [key: string]: unknown;
    };
    favourite_count?: number;
    favorites_count?: number;
    view_count?: number;
    availability?: string;
    [key: string]: unknown;
}

export function getVintedItems(response: VintedSearchResponse): VintedItem[] {
    if (Array.isArray(response.items)) {
        return response.items;
    }

    if (Array.isArray(response.data?.items)) {
        return response.data.items;
    }

    return [];
}

export function getVintedPagination(response: VintedSearchResponse): VintedPagination | undefined {
    return response.pagination ?? response.data?.pagination;
}

function label(value: unknown): string | null {
    if (typeof value === 'string' && value.trim() !== '') {
        return value;
    }

    if (value && typeof value === 'object') {
        const record = value as Record<string, unknown>;
        for (const key of ['title', 'name']) {
            if (typeof record[key] === 'string' && record[key] !== '') {
                return record[key];
            }
        }
    }

    return null;
}

function moneyAmount(value: VintedItem['price'] | VintedItem['total_item_price']): number | string | null {
    if (typeof value === 'number' || typeof value === 'string') {
        return value;
    }

    return value?.amount ?? null;
}

function moneyCurrency(value: VintedItem['price'] | VintedItem['total_item_price']): string | null {
    if (value && typeof value === 'object' && typeof value.currency === 'string') {
        return value.currency;
    }

    if (value && typeof value === 'object' && typeof value.currency_code === 'string') {
        return value.currency_code;
    }

    return null;
}

function firstPhotoUrl(item: VintedItem): string | null {
    if (item.image_url) {
        return item.image_url;
    }

    if (item.photo_url) {
        return item.photo_url;
    }

    if (item.photo?.url) {
        return item.photo.url;
    }

    const firstPhoto = Array.isArray(item.photos) ? item.photos.find((photo) => photo?.url) : null;
    return firstPhoto?.url ?? null;
}

export function buildVintedDatasetItem(
    item: VintedItem,
    params: Record<string, unknown>,
    response: VintedSearchResponse,
): Record<string, unknown> {
    const pagination = getVintedPagination(response);
    const seller = item.seller && typeof item.seller === 'object'
        ? item.seller
        : item.user && typeof item.user === 'object'
            ? item.user
            : {};

    return {
        ...item,
        id: item.id ?? null,
        title: item.title ?? null,
        description: item.description ?? null,
        url: item.url ?? null,
        path: item.path ?? null,
        image_url: firstPhotoUrl(item),
        price_amount: moneyAmount(item.price),
        price_currency: moneyCurrency(item.price),
        total_item_price: moneyAmount(item.total_item_price),
        total_item_price_currency: moneyCurrency(item.total_item_price),
        shipping_price: moneyAmount(item.shipping_price),
        shipping_price_currency: moneyCurrency(item.shipping_price),
        service_fee: moneyAmount(item.service_fee),
        service_fee_currency: moneyCurrency(item.service_fee),
        brand_name: label(item.brand) ?? item.brand_title ?? null,
        category_name: label(item.category),
        size_name: label(item.size) ?? item.size_title ?? null,
        color_name: label(item.color) ?? item.color_title ?? null,
        condition: item.condition ?? item.condition_title ?? item.status ?? item.status_title ?? null,
        availability: item.availability ?? null,
        favourite_count: item.favourite_count ?? item.favorites_count ?? null,
        view_count: item.view_count ?? null,
        seller_id: seller.id ?? null,
        seller_login: seller.login ?? seller.username ?? null,
        seller_feedback_count: seller.feedback_count ?? null,
        seller_feedback_reputation: seller.feedback_reputation ?? null,
        request_query: params.query ?? null,
        request_country: params.country ?? null,
        request_page: params.page ?? null,
        request_per_page: params.per_page ?? null,
        request_order: params.order ?? null,
        request_brand_ids: params.brand_ids ?? null,
        request_catalog_ids: params.catalog_ids ?? null,
        request_size_ids: params.size_ids ?? null,
        request_price_from: params.price_from ?? null,
        request_price_to: params.price_to ?? null,
        total_pages: pagination?.total_pages ?? null,
        total_entries: pagination?.total_entries ?? pagination?.total_items ?? null,
    };
}
