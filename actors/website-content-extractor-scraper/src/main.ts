import { Actor } from 'apify';
import { getInputUrls, getResponseType, type WebsiteContentExtractorInput } from './input.js';
import { buildWebScraperParams, describeWebScraperRequest } from './request-params.js';
import {
    buildFailureDatasetItem,
    buildJsonDatasetItem,
    buildMarkdownDatasetItem,
    isSuccessfulDatasetItem,
    type WebScraperDatasetItem,
    type WebScraperJsonResponse,
} from './response-utils.js';
import { ScrappaWebScraperClient } from './web-scraper-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const URL_RESULT_CHARGE_EVENT = 'url-result';

async function pushDatasetItem(item: WebScraperDatasetItem): Promise<boolean> {
    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent || !isSuccessfulDatasetItem(item)) {
        await Actor.pushData(item);
        return true;
    }

    const chargeResult = await Actor.pushData(item, URL_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        console.log('Charge limit reached while saving the website content result.', JSON.stringify({
            event: URL_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            input_url: item.input_url,
        }));
        return false;
    }

    return true;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<WebsiteContentExtractorInput>();
        const responseType = getResponseType(input);
        const requests = getInputUrls(input);
        if (requests.length === 0) {
            throw new Error('At least one URL is required. Provide urls or backward-compatible url.');
        }

        const client = new ScrappaWebScraperClient({
            apiKey,
            timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS,
        });

        let succeeded = 0;
        let failed = 0;
        let saved = 0;

        console.log(`Extracting website content for ${requests.length} URL${requests.length === 1 ? '' : 's'} with response_type=${responseType}`);

        for (const request of requests) {
            const params = buildWebScraperParams(request, input, responseType);
            console.log(`Calling Scrappa Web Scraper API (${describeWebScraperRequest(params)})`);

            let item: WebScraperDatasetItem;
            try {
                if (responseType === 'markdown') {
                    const markdown = await client.scrapeMarkdown(params);
                    item = buildMarkdownDatasetItem(markdown, request, params);
                } else {
                    const response = await client.scrapeJson<WebScraperJsonResponse>(params);
                    item = buildJsonDatasetItem(response, request, params);
                }
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`Scrappa Web Scraper API returned a per-URL failure for ${request.input_url}: ${message}`);
                item = buildFailureDatasetItem(error, request, params);
            }

            const savedItem = await pushDatasetItem(item);
            if (!savedItem) {
                break;
            }

            saved += 1;
            if (item.success) {
                succeeded += 1;
            } else {
                failed += 1;
            }
        }

        const summary = {
            requested: requests.length,
            saved,
            succeeded,
            failed,
            response_type: responseType,
        };
        console.log('Website content extraction completed');
        console.log('Results summary:', JSON.stringify(summary));

        if (saved < requests.length) {
            await Actor.exit({
                statusMessage: `Saved ${saved} of ${requests.length} requested URL result(s).`,
            });
            return;
        }
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
