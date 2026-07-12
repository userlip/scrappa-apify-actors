import type { PriceInsightsRequest } from './request-params.js';

export interface PriceInsightsResponse {
    success?: boolean;
    location?: unknown;
    geocode?: unknown;
    currency?: unknown;
    prices?: {
        apartment_rent_per_m2?: unknown;
        apartment_buy_per_m2?: unknown;
        house_rent_per_m2?: unknown;
        house_buy_per_m2?: unknown;
    } | null;
}

export function buildPriceInsightItem(
    response: PriceInsightsResponse,
    request: PriceInsightsRequest,
): Record<string, unknown> | null {
    const prices = response.prices;
    const location = cleanString(response.location);
    const geocode = cleanString(response.geocode);
    const currency = cleanString(response.currency);

    if (response.success !== true || !prices || !location || !geocode || !currency) {
        return null;
    }

    const apartmentRent = cleanNumber(prices.apartment_rent_per_m2);
    const apartmentBuy = cleanNumber(prices.apartment_buy_per_m2);
    const houseRent = cleanNumber(prices.house_rent_per_m2);
    const houseBuy = cleanNumber(prices.house_buy_per_m2);
    if ([apartmentRent, apartmentBuy, houseRent, houseBuy].some((price) => price === null)) {
        return null;
    }

    return {
        location,
        geocode,
        currency,
        apartment_rent_per_m2: apartmentRent,
        apartment_buy_per_m2: apartmentBuy,
        house_rent_per_m2: houseRent,
        house_buy_per_m2: houseBuy,
        request_location: request.location,
        request_index: request.index,
    };
}

function cleanString(value: unknown): string | null {
    return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function cleanNumber(value: unknown): number | null {
    if (typeof value !== 'number' && typeof value !== 'string') {
        return null;
    }

    const number = typeof value === 'string' && value.trim() ? Number(value) : value;
    return typeof number === 'number' && Number.isFinite(number) && number > 0 ? number : null;
}
