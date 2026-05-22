import { pathToFileURL } from 'node:url';
import { Actor } from 'apify';
import {
    buildImmobilienscout24SearchParams,
    describeImmobilienscout24SearchRequest,
    normalizeImmobilienscout24SearchInput,
} from './request-params.js';
import type { Immobilienscout24SearchInput } from './request-params.js';
import {
    buildImmobilienscout24DatasetItem,
    getImmobilienscout24Listings,
    getImmobilienscout24Page,
    getImmobilienscout24TotalPages,
    getImmobilienscout24TotalResults,
    limitImmobilienscout24SearchResponse,
} from './response-utils.js';
import type { Immobilienscout24SearchResponse } from './response-utils.js';
import { ScrappaApiError, ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = normalizeImmobilienscout24SearchInput(await Actor.getInput<Immobilienscout24SearchInput>());
        const params = buildImmobilienscout24SearchParams(input);
        console.log(`Searching ImmobilienScout24 for ${describeImmobilienscout24SearchRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await searchImmobilienscout24(client, params);
        const rawListings = getImmobilienscout24Listings(response);
        const requestedLimit = params.per_page as number;
        const listings = rawListings
            .slice(0, requestedLimit)
            .map((listing) => buildImmobilienscout24DatasetItem(listing, params));

        if (listings.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(listings, PROPERTY_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < listings.length) {
                    const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${listings.length} ImmobilienScout24 property result(s); OUTPUT was not written.`;
                    console.log(statusMessage, JSON.stringify({
                        event: PROPERTY_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: listings.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(listings);
            }

            console.log(`Found ${listings.length} ImmobilienScout24 property result(s)`);
            if (rawListings.length > listings.length) {
                console.log(`Scrappa returned ${rawListings.length} result(s); saved the requested limit of ${listings.length}.`);
            }
        } else {
            console.log('No ImmobilienScout24 property results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', limitImmobilienscout24SearchResponse(response, listings.length));

        console.log('ImmobilienScout24 property search completed successfully');
        console.log('Results summary:', JSON.stringify({
            listings: listings.length,
            total_results: getImmobilienscout24TotalResults(response),
            page: getImmobilienscout24Page(response),
            total_pages: getImmobilienscout24TotalPages(response),
            request_location: params.location,
            request_type: params.type,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The ImmobilienScout24 request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a smaller page size or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

async function searchImmobilienscout24(
    client: ScrappaClient,
    params: Record<string, unknown>,
): Promise<Immobilienscout24SearchResponse> {
    try {
        return await client.get<Immobilienscout24SearchResponse>('/immobilienscout24/search', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
    } catch (error) {
        if (!isHandledEmptySearchError(error)) {
            throw error;
        }

        const message = error instanceof ScrappaApiError
            ? error.responseMessage
            : error instanceof Error ? error.message : String(error);

        console.warn('Scrappa ImmobilienScout24 search returned no usable result; saving a clean zero-result output.', JSON.stringify({
            status: error instanceof ScrappaApiError ? error.status : null,
            message,
            request_location: params.location,
            request_type: params.type,
        }));

        return {
            success: false,
            total_results: 0,
            page: getSearchPage(params),
            total_pages: 0,
            results: [],
            error: {
                message,
                status: error instanceof ScrappaApiError ? error.status : null,
            },
        };
    }
}

export function isHandledEmptySearchError(error: unknown): boolean {
    if (!(error instanceof ScrappaApiError)) {
        return false;
    }

    if (error.status === 400 && /invalid_location|location .*not found/i.test(error.responseMessage)) {
        return true;
    }

    return error.status === 502;
}

function getSearchPage(params: Record<string, unknown>): number {
    return typeof params.page === 'number' ? params.page : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    main().catch((error) => {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        process.exitCode = 1;
    });
}
