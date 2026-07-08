import type { VintedItemDetailsRequest } from './request-params.js';

export interface VintedItemDetailsResponse {
    success?: boolean;
    data?: VintedItemDetails | { item?: VintedItemDetails; [key: string]: unknown };
    item?: VintedItemDetails;
    message?: string;
    status_code?: number;
    [key: string]: unknown;
}

export interface VintedItemDetails {
    id?: string | number;
    title?: string;
    description?: string;
    price?: MoneyValue;
    total_item_price?: MoneyValue;
    shipping_price?: MoneyValue;
    service_fee?: MoneyValue;
    brand?: LabelValue;
    brand_title?: string;
    category?: LabelValue;
    size?: LabelValue;
    size_title?: string;
    color?: LabelValue;
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
    seller?: SellerValue;
    user?: SellerValue;
    favourite_count?: number;
    favorites_count?: number;
    view_count?: number;
    availability?: string;
    created_at?: string;
    updated_at?: string;
    [key: string]: unknown;
}

type MoneyValue = { amount?: number | string; currency?: string; currency_code?: string; [key: string]: unknown } | number | string;
type LabelValue = string | { title?: string; name?: string; [key: string]: unknown };
type SellerValue = {
    id?: string | number;
    login?: string;
    username?: string;
    feedback_count?: number;
    feedback_reputation?: number;
    [key: string]: unknown;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function label(value: unknown): string | null {
    if (typeof value === 'string' && value.trim() !== '') {
        return value;
    }

    if (isRecord(value)) {
        for (const key of ['title', 'name']) {
            if (typeof value[key] === 'string' && value[key] !== '') {
                return value[key];
            }
        }
    }

    return null;
}

function moneyAmount(value: MoneyValue | undefined): number | string | null {
    if (typeof value === 'number' || typeof value === 'string') {
        return value;
    }

    return value?.amount ?? null;
}

function moneyCurrency(value: MoneyValue | undefined): string | null {
    if (value && typeof value === 'object' && typeof value.currency === 'string') {
        return value.currency;
    }

    if (value && typeof value === 'object' && typeof value.currency_code === 'string') {
        return value.currency_code;
    }

    return null;
}

function firstPhotoUrl(item: VintedItemDetails): string | null {
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

export function getVintedItemDetails(response: VintedItemDetailsResponse): VintedItemDetails {
    if (isRecord(response.item)) {
        return response.item as VintedItemDetails;
    }

    if (isRecord(response.data) && isRecord(response.data.item)) {
        return response.data.item as VintedItemDetails;
    }

    if (isRecord(response.data)) {
        return response.data as VintedItemDetails;
    }

    return response as VintedItemDetails;
}

export function buildVintedItemDetailsDatasetItem(
    details: VintedItemDetails,
    request: VintedItemDetailsRequest,
): Record<string, unknown> {
    const seller = isRecord(details.seller)
        ? details.seller
        : isRecord(details.user)
            ? details.user
            : {};

    return {
        ...details,
        id: details.id ?? request.itemId,
        title: details.title ?? null,
        description: details.description ?? null,
        url: details.url ?? null,
        path: details.path ?? null,
        image_url: firstPhotoUrl(details),
        price_amount: moneyAmount(details.price),
        price_currency: moneyCurrency(details.price),
        total_item_price: moneyAmount(details.total_item_price),
        total_item_price_currency: moneyCurrency(details.total_item_price),
        shipping_price: moneyAmount(details.shipping_price),
        shipping_price_currency: moneyCurrency(details.shipping_price),
        service_fee: moneyAmount(details.service_fee),
        service_fee_currency: moneyCurrency(details.service_fee),
        brand_name: label(details.brand) ?? details.brand_title ?? null,
        category_name: label(details.category),
        size_name: label(details.size) ?? details.size_title ?? null,
        color_name: label(details.color) ?? details.color_title ?? null,
        condition: details.condition ?? details.condition_title ?? details.status ?? details.status_title ?? null,
        availability: details.availability ?? null,
        favourite_count: details.favourite_count ?? details.favorites_count ?? null,
        view_count: details.view_count ?? null,
        seller_id: seller.id ?? null,
        seller_login: seller.login ?? seller.username ?? null,
        seller_feedback_count: seller.feedback_count ?? null,
        seller_feedback_reputation: seller.feedback_reputation ?? null,
        request_item_id: request.itemId,
        request_country: request.params.country,
        request_index: request.index,
        request_success: true,
    };
}

export function buildVintedItemDetailsErrorItem(
    request: VintedItemDetailsRequest,
    error: unknown,
): Record<string, unknown> {
    return {
        id: request.itemId,
        request_item_id: request.itemId,
        request_country: request.params.country,
        request_index: request.index,
        request_success: false,
        error_message: error instanceof Error ? error.message : String(error),
    };
}
