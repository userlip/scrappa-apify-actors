import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import {
    buildTikTokVideoParams,
    formatTikTokVideoLookupForLog,
    resolveTikTokVideoLookups,
} from './request-params.js';
import type { TikTokVideoInput } from './request-params.js';

interface TikTokAuthor {
    id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    [key: string]: unknown;
}

interface TikTokVideo {
    aweme_id?: string;
    id?: string;
    region?: string;
    title?: string;
    cover?: string;
    duration?: number;
    play?: string;
    wmplay?: string;
    hdplay?: string | null;
    size?: number;
    wm_size?: number;
    hd_size?: number | null;
    music?: string;
    music_info?: Record<string, unknown>;
    play_count?: number;
    digg_count?: number;
    comment_count?: number;
    share_count?: number;
    download_count?: number;
    collect_count?: number;
    create_time?: number;
    author?: TikTokAuthor;
    [key: string]: unknown;
}

interface TikTokVideoResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokVideo | TikTokVideo[] | null;
    [key: string]: unknown;
}

interface TikTokVideoDatasetItem extends TikTokVideo {
    request_url: string;
    request_hd: boolean;
    request_index: number;
    result_found: boolean;
    processed_time: number | null;
    error_message?: string;
}

function assertSuccessfulResponse(response: TikTokVideoResponse, url: string): void {
    if (response.code !== undefined && response.code !== 0) {
        throw new Error(`Scrappa TikTok Video API returned code ${response.code} for ${url}: ${response.msg ?? 'Unknown error'}`);
    }
}

function extractVideo(data: TikTokVideoResponse['data'], url: string): TikTokVideo | null {
    if (!data) {
        return null;
    }

    if (Array.isArray(data)) {
        if (data.length > 1) {
            console.warn(`Scrappa returned ${data.length} video records for ${url}. Saving the first record to keep one dataset item per requested URL.`);
        }
        return data[0] ?? null;
    }

    return data;
}

function toDatasetItem(
    video: TikTokVideo,
    url: string,
    input: TikTokVideoInput,
    response: TikTokVideoResponse,
    requestIndex: number,
): TikTokVideoDatasetItem {
    return {
        ...video,
        request_url: url,
        request_hd: input.hd === true,
        request_index: requestIndex,
        result_found: true,
        processed_time: response.processed_time ?? null,
    };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokVideoInput>();
        if (!input) {
            throw new Error('At least one TikTok video URL is required');
        }

        const urls = resolveTikTokVideoLookups(input);
        const client = new ScrappaClient({ apiKey });
        const rows: TikTokVideoDatasetItem[] = [];

        console.log(`Fetching TikTok video details for ${urls.length} URL${urls.length === 1 ? '' : 's'}`);

        for (const [index, url] of urls.entries()) {
            const requestIndex = index + 1;

            try {
                const params = buildTikTokVideoParams(url, input);
                console.log(`Fetching TikTok video ${requestIndex}/${urls.length}: ${formatTikTokVideoLookupForLog(url)}`);

                const response = await client.get<TikTokVideoResponse>('/tiktok/video', params);
                assertSuccessfulResponse(response, url);

                const video = extractVideo(response.data, url);
                const row = video
                    ? toDatasetItem(video, url, input, response, requestIndex)
                    : {
                        request_url: url,
                        request_hd: input.hd === true,
                        request_index: requestIndex,
                        result_found: false,
                        processed_time: response.processed_time ?? null,
                    };

                await Actor.pushData(row);
                rows.push(row);

                if (video) {
                    console.log('Found 1 TikTok video record');
                } else {
                    console.log(`No video details found for: ${formatTikTokVideoLookupForLog(url)}`);
                }
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`TikTok video lookup failed for ${formatTikTokVideoLookupForLog(url)}: ${message}`);

                const row: TikTokVideoDatasetItem = {
                    request_url: url,
                    request_hd: input.hd === true,
                    request_index: requestIndex,
                    result_found: false,
                    processed_time: null,
                    error_message: message,
                };

                await Actor.pushData(row);
                rows.push(row);
            }
        }

        const summary = {
            urls_requested: urls.length,
            dataset_items: rows.length,
            videos_found: rows.filter((row) => row.result_found).length,
            lookups_failed: rows.filter((row) => row.error_message).length,
            hd_requested: input.hd === true,
        };

        console.log('TikTok video details extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
