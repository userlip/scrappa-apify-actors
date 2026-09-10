import { Actor } from 'apify';
import { buildKleinanzeigenDetailsPlan, describeKleinanzeigenDetailsRequest, getDiscoveryQuery, planDiscoveredListings } from './request-params.js';
import type { KleinanzeigenDetailsInput } from './request-params.js';
import { buildListingDetailsOutput, processKleinanzeigenListingDetails } from './listing-processing.js';
import type { KleinanzeigenDetailsResponse } from './response-utils.js';
import { errorSummary, ScrappaClient } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
async function main(): Promise<void> {
    await Actor.init();
    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        const input = await Actor.getInput<KleinanzeigenDetailsInput>() ?? {};
        const query = getDiscoveryQuery(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: query ? 60000 : SCRAPPA_REQUEST_TIMEOUT_MS });
        const plan = query
            ? planDiscoveredListings(await client.get('/kleinanzeigen/search', { query, page: 1 }, { attempts: 1 }))
            : buildKleinanzeigenDetailsPlan(input);
        console.log(`Fetching ${describeKleinanzeigenDetailsRequest(plan)}`);
        const result = await processKleinanzeigenListingDetails(Actor, plan.listings, (adId) => (
            client.get<KleinanzeigenDetailsResponse>(
                '/kleinanzeigen/details',
                { ad_id: adId },
                { attempts: query ? 1 : SCRAPPA_MAX_ATTEMPTS },
            )
        ), query ? 1 : Infinity);
        await (await Actor.openKeyValueStore()).setValue('OUTPUT', buildListingDetailsOutput(plan.listings.length, result));
        if (result.savedCount === 0 && result.failures.length > 0) {
            throw new Error(`All ${result.completedCount} requested Kleinanzeigen listing detail request(s) failed.`);
        }
        if (result.statusMessage) { await Actor.exit({ statusMessage: result.statusMessage }); return; }
    } catch (error) {
        const message = errorSummary(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }
    await Actor.exit();
}

main().catch((error) => { console.error('Actor failed: ' + errorSummary(error)); process.exitCode = 1; });
