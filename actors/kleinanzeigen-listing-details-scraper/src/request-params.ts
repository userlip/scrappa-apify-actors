export interface KleinanzeigenDetailsInput {
    ad_id?: unknown;
    ad_ids?: unknown;
}

export interface KleinanzeigenDetailsPlanItem {
    adId: string;
    index: number;
}

export interface KleinanzeigenDetailsPlan {
    listings: KleinanzeigenDetailsPlanItem[];
}

export const MAX_BATCH_AD_IDS = 100;

function normalizeAdId(value: unknown, field: string): string | undefined {
    if (value === undefined || value === null) return undefined;
    if (typeof value === 'number') {
        if (!Number.isSafeInteger(value) || value < 0) throw new Error(`${field} must be a safe integer or string`);
        return String(value);
    }
    if (typeof value !== 'string') throw new Error(`${field} must be a string or safe integer`);
    const trimmed = value.trim();
    if (!trimmed) throw new Error(`${field} cannot be blank`);
    if (!/^\d+$/.test(trimmed)) throw new Error(`${field} must contain only digits`);
    return trimmed;
}

export function buildKleinanzeigenDetailsPlan(input: KleinanzeigenDetailsInput = {}): KleinanzeigenDetailsPlan {
    if (input.ad_ids !== undefined && !Array.isArray(input.ad_ids)) throw new Error('ad_ids must be an array');
    const rawIds: Array<{ value: unknown; field: string }> = [];
    if (input.ad_id !== undefined) rawIds.push({ value: input.ad_id, field: 'ad_id' });
    if (Array.isArray(input.ad_ids)) {
        input.ad_ids.forEach((value, index) => rawIds.push({ value, field: `ad_ids[${index}]` }));
    }
    const uniqueIds: string[] = [];
    const seen = new Set<string>();
    for (const raw of rawIds) {
        const adId = normalizeAdId(raw.value, raw.field);
        if (adId && !seen.has(adId)) { seen.add(adId); uniqueIds.push(adId); }
    }
    if (!uniqueIds.length) throw new Error('Provide ad_id or at least one ad_ids entry');
    if (uniqueIds.length > MAX_BATCH_AD_IDS) throw new Error(`A maximum of ${MAX_BATCH_AD_IDS} unique ad IDs is allowed`);
    return { listings: uniqueIds.map((adId, index) => ({ adId, index })) };
}

export function describeKleinanzeigenDetailsRequest(plan: KleinanzeigenDetailsPlan): string {
    return plan.listings.length === 1 ? `listing ${plan.listings[0]?.adId}` : `${plan.listings.length} Kleinanzeigen listings`;
}
