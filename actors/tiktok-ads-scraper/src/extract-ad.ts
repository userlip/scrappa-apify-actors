import type { TikTokAdRecord } from './normalize-ad.js';

export type TikTokAdResponseData = TikTokAdRecord | TikTokAdRecord[] | null | undefined;

export function extractSingleTikTokAdRecord(
    data: TikTokAdResponseData,
    url: string,
    warn: (message: string) => void = console.warn,
): TikTokAdRecord | null {
    if (!data) {
        return null;
    }

    if (!Array.isArray(data)) {
        return data;
    }

    if (data.length === 0) {
        warn(`Scrappa returned an empty ad record array for ${url}. Saving a not-found dataset item.`);
        return null;
    }

    if (data.length > 1) {
        throw new Error(`Scrappa returned ${data.length} ad records for ${url}. Expected exactly one ad record for a single Creative Center ad URL.`);
    }

    return data[0];
}
