export interface RedfinValuationResponse {
    data?: Record<string, unknown> | null;
    [key: string]: unknown;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
        if (typeof value === 'string' && value.trim() !== '') {
            const number = Number(value);
            if (Number.isFinite(number)) {
                return number;
            }
        }
    }

    return null;
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
        if (typeof value === 'number' && Number.isFinite(value)) {
            return String(value);
        }
    }

    return null;
}

function nestedValue(record: Record<string, unknown>, key: string): unknown {
    const value = record[key];
    if (typeof value === 'object' && value !== null && 'value' in value) {
        return (value as Record<string, unknown>).value;
    }

    return value;
}

export function getRedfinValuationData(response: RedfinValuationResponse): Record<string, unknown> {
    if (typeof response.data === 'object' && response.data !== null && !Array.isArray(response.data)) {
        return response.data;
    }

    return response;
}

export function hasMeaningfulValuationData(data: Record<string, unknown>): boolean {
    return firstNumber(
        data.predictedValue,
        data.predicted_value,
        data.predictedValueLow,
        data.predicted_value_low,
        data.predictedValueHigh,
        data.predicted_value_high,
        data.lastSoldPrice,
        data.last_sold_price,
    ) !== null;
}

export function buildRedfinValuationDatasetItem(
    response: RedfinValuationResponse,
    request: {
        property_id: number;
        listing_id: number | null;
        url: string | null;
        index: number;
    },
): Record<string, unknown> {
    const data = getRedfinValuationData(response);
    const comparables = Array.isArray(data.comparables) ? data.comparables : [];

    return {
        ...data,
        property_id: request.property_id,
        listing_id: request.listing_id,
        predicted_value: firstNumber(data.predictedValue, data.predicted_value),
        predicted_value_low: firstNumber(data.predictedValueLow, data.predicted_value_low),
        predicted_value_high: firstNumber(data.predictedValueHigh, data.predicted_value_high),
        last_sold_price: firstNumber(data.lastSoldPrice, data.last_sold_price),
        last_sold_date: firstString(data.lastSoldDate, data.last_sold_date),
        beds: firstNumber(data.numBeds, data.beds),
        baths: firstNumber(data.numBaths, data.baths),
        sqft: firstNumber(nestedValue(data, 'sqFt'), data.sqft),
        lot_size: firstNumber(nestedValue(data, 'lotSize'), data.lot_size),
        year_built: firstNumber(data.yearBuilt, data.year_built),
        comparables_count: comparables.length,
        comparables,
        request_index: request.index,
        request_property_id: request.property_id,
        request_listing_id: request.listing_id,
        request_url: request.url,
    };
}
