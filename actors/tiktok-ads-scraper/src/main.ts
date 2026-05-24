import { Actor } from 'apify';
import { normalizeTikTokAdRecord } from './normalize-ad.js';
import type { TikTokAdRecord } from './normalize-ad.js';
import { ScrappaClient } from './shared/scrappa-client.js';
import {
    buildTikTokAdParams,
    extractTikTokAdId,
    formatTikTokAdLookupForLog,
    resolveTikTokAdRequests,
} from './request-params.js';
import type { TikTokAdsInput } from './request-params.js';

interface TikTokAdResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    cached?: boolean;
    data?: TikTokAdRecord | TikTokAdRecord[] | null;
    [key: string]: unknown;
}

interface TikTokAdDatasetItem extends TikTokAdRecord {
    request_url: string;
    request_ad_id: string | null;
    request_index: number;
    result_found: boolean;
    processed_time: number | null;
    cached: boolean | null;
    error_message?: string;
}

function assertSuccessfulResponse(response: TikTokAdResponse, url: string): void {
    if (response.code !== undefined && response.code !== 0) {
        throw new Error(`Scrappa TikTok Ads API returned code ${response.code} for ${url}: ${response.msg ?? 'Unknown error'}`);
    }
}

function extractAd(data: TikTokAdResponse['data'], url: string): TikTokAdRecord | null {
    if (!data) {
        return null;
    }

    if (Array.isArray(data)) {
        if (data.length === 0) {
            console.warn(`Scrappa returned an empty ad record array for ${url}. Saving a not-found dataset item.`);
            return null;
        }

        if (data.length > 1) {
            console.warn(`Scrappa returned ${data.length} ad records for ${url}. Saving the first record to keep one dataset item per requested URL.`);
        }

        return data[0];
    }

    return data;
}

function formatLookupForLog(value: string): string {
    try {
        return formatTikTokAdLookupForLog(value);
    } catch {
        return value.trim();
    }
}

function toDatasetItem(
    ad: TikTokAdRecord,
    url: string,
    response: TikTokAdResponse,
    requestIndex: number,
): TikTokAdDatasetItem {
    return {
        ...normalizeTikTokAdRecord(ad),
        request_url: url,
        request_ad_id: extractTikTokAdId(url),
        request_index: requestIndex,
        result_found: true,
        processed_time: response.processed_time ?? null,
        cached: typeof response.cached === 'boolean' ? response.cached : null,
    };
}

async function main(): Promise<void> {
    try {
        await Actor.init();

        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokAdsInput>();
        if (!input) {
            throw new Error('At least one TikTok Creative Center ad URL is required');
        }

        const requests = resolveTikTokAdRequests(input);
        const client = new ScrappaClient({ apiKey });
        let datasetItems = 0;
        let adsFound = 0;
        let lookupsFailed = 0;

        console.log(`Fetching TikTok ad details for ${requests.length} URL${requests.length === 1 ? '' : 's'}`);

        for (const [index, request] of requests.entries()) {
            const requestIndex = index + 1;
            const { url } = request;

            try {
                if (request.validationError) {
                    throw new Error(request.validationError);
                }

                const params = buildTikTokAdParams(url);
                console.log(`Fetching TikTok ad ${requestIndex}/${requests.length}: ${formatLookupForLog(url)}`);

                const response = await client.get<TikTokAdResponse>('/tiktok/ads/details', params);
                assertSuccessfulResponse(response, url);

                const ad = extractAd(response.data, url);
                const row = ad
                    ? toDatasetItem(ad, url, response, requestIndex)
                    : {
                        request_url: url,
                        request_ad_id: extractTikTokAdId(url),
                        request_index: requestIndex,
                        result_found: false,
                        processed_time: response.processed_time ?? null,
                        cached: typeof response.cached === 'boolean' ? response.cached : null,
                    };

                await Actor.pushData(row);
                datasetItems += 1;

                if (ad) {
                    adsFound += 1;
                    console.log('Found 1 TikTok ad record');
                } else {
                    console.log(`No ad details found for: ${formatLookupForLog(url)}`);
                }
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`TikTok ad lookup failed for ${formatLookupForLog(url)}: ${message}`);

                const row: TikTokAdDatasetItem = {
                    request_url: url,
                    request_ad_id: extractTikTokAdId(url),
                    request_index: requestIndex,
                    result_found: false,
                    processed_time: null,
                    cached: null,
                    error_message: message,
                };

                await Actor.pushData(row);
                datasetItems += 1;
                lookupsFailed += 1;
            }
        }

        const summary = {
            urls_requested: requests.length,
            dataset_items: datasetItems,
            ads_found: adsFound,
            lookups_failed: lookupsFailed,
        };

        console.log('TikTok ad details extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));

        await Actor.exit();
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
