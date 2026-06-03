import { Actor } from 'apify';
import { buildGoogleNewsParamList, describeGoogleNewsRequest } from './request-params.js';
import type { GoogleNewsInput } from './request-params.js';
import { ScrappaClient } from './shared/scrappa-client.js';

interface GoogleNewsSource {
    name?: string;
    title?: string;
    icon?: string;
    authors?: string[];
    [key: string]: unknown;
}

interface GoogleNewsResult {
    position?: number;
    title?: string;
    link?: string;
    type?: string;
    source?: GoogleNewsSource | string;
    authors?: string[];
    date?: string;
    iso_date?: string;
    published_at?: string;
    thumbnail?: string;
    thumbnail_small?: string;
    snippet?: string;
    story_token?: string;
    [key: string]: unknown;
}

interface GoogleNewsResponse {
    news_results?: GoogleNewsResult[];
    menu_links?: unknown[];
    related_publications?: unknown[];
    sub_menu_links?: unknown[];
    highlight?: unknown;
    related_topics?: unknown[];
    related_searches?: unknown[];
    stories?: unknown[];
    [key: string]: unknown;
}

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function sourceName(source: GoogleNewsSource | string | undefined): string | undefined {
    if (typeof source === 'string') {
        return source;
    }

    return source?.name ?? source?.title;
}

function enrichResult(result: GoogleNewsResult, params: Record<string, unknown>): Record<string, unknown> {
    return {
        ...result,
        source_name: sourceName(result.source),
        request_q: params.q ?? null,
        request_gl: params.gl ?? null,
        request_hl: params.hl ?? null,
        request_page: params.page ?? null,
        request_start: params.start ?? null,
        request_so: params.so ?? null,
        request_topic_token: params.topic_token ?? null,
        request_publication_token: params.publication_token ?? null,
        request_section_token: params.section_token ?? null,
        request_story_token: params.story_token ?? null,
        request_kgmid: params.kgmid ?? null,
    };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleNewsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const paramList = buildGoogleNewsParamList(input);
        console.log(`Running ${paramList.length} Google News request${paramList.length === 1 ? '' : 's'}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const keepRawResponse = paramList.length === 1;
        let singleResponse: GoogleNewsResponse | null = null;
        const requestSummaries: Array<{
            request: Record<string, unknown>;
            success: boolean;
            news_results: number;
            stories: number;
            related_searches: number;
            error_message?: string;
        }> = [];
        let totalNewsResults = 0;
        let failedRequests = 0;

        for (const params of paramList) {
            const requestDescription = describeGoogleNewsRequest(params);
            console.log(`Fetching Google News for ${requestDescription}`);

            try {
                const response = await client.get<GoogleNewsResponse>('/google/news', params);
                if (keepRawResponse) {
                    singleResponse = response;
                }
                const newsResults = response.news_results ?? [];
                totalNewsResults += newsResults.length;

                if (newsResults.length > 0) {
                    await Actor.pushData(newsResults.map((result) => enrichResult(result, params)));
                    console.log(`Found ${newsResults.length} news results`);
                } else {
                    console.log('No Google News results found for this request');
                }

                requestSummaries.push({
                    request: params,
                    success: true,
                    news_results: newsResults.length,
                    stories: response.stories?.length ?? 0,
                    related_searches: response.related_searches?.length ?? 0,
                });
            } catch (error) {
                if (paramList.length === 1) {
                    throw error;
                }

                failedRequests += 1;
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`Google News request failed for ${requestDescription}: ${message}`);
                requestSummaries.push({
                    request: params,
                    success: false,
                    news_results: 0,
                    stories: 0,
                    related_searches: 0,
                    error_message: message,
                });
            }
        }

        const store = await Actor.openKeyValueStore();
        if (keepRawResponse) {
            await store.setValue('OUTPUT', singleResponse);
        } else {
            await store.setValue('OUTPUT', {
                requests: requestSummaries,
                news_results: totalNewsResults,
                failed_requests: failedRequests,
            });
        }

        const summary = {
            requests: paramList.length,
            news_results: totalNewsResults,
            failed_requests: failedRequests,
        };

        console.log('Google News scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Google News request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific query or run the request again.`
            : rawMessage;
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
