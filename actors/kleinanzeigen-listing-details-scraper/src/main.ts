import { Actor } from 'apify';
import { buildKleinanzeigenDetailsPlan, describeKleinanzeigenDetailsRequest } from './request-params.js';
import type { KleinanzeigenDetailsInput } from './request-params.js';
import { buildKleinanzeigenDetailsDatasetItem, selectKleinanzeigenDetail } from './response-utils.js';
import type { KleinanzeigenDetailsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
export const LISTING_DETAIL_RESULT_CHARGE_EVENT = 'listing-detail-result';

function getChargeableListingCapacity(): number {
    const manager = Actor.getChargingManager();
    return manager.getPricingInfo().isPayPerEvent
        ? manager.calculateMaxEventChargeCountWithinLimit(LISTING_DETAIL_RESULT_CHARGE_EVENT)
        : Infinity;
}

function errorSummary(error: unknown): string {
    const message = error instanceof Error ? error.message : String(error);
    return message.replace(/(?:X-API-Key|Authorization)\s*[:=]\s*\S+/gi, '[redacted]').slice(0, 500);
}

async function main(): Promise<void> {
    await Actor.init();
    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        const plan = buildKleinanzeigenDetailsPlan(await Actor.getInput<KleinanzeigenDetailsInput>() ?? {});
        console.log(`Fetching ${describeKleinanzeigenDetailsRequest(plan)}`);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const failures: Array<{ ad_id: string; error: string; outcome: 'failed' }> = [];
        let savedCount = 0;
        let completedCount = 0;
        let statusMessage: string | null = null;

        for (const listing of plan.listings) {
            if (getChargeableListingCapacity() <= 0) {
                statusMessage = `Charge limit reached before fetching Kleinanzeigen listing ${listing.adId}.`;
                console.log(statusMessage);
                break;
            }
            completedCount++;
            try {
                const response = await client.get<KleinanzeigenDetailsResponse>('/kleinanzeigen/details', { ad_id: listing.adId }, { attempts: SCRAPPA_MAX_ATTEMPTS });
                const detail = selectKleinanzeigenDetail(response);
                if (!detail) throw new Error('Scrappa response did not contain a recognizable listing detail payload');
                const chargeResult = await Actor.pushData(
                    buildKleinanzeigenDetailsDatasetItem(detail, listing.adId, listing.index),
                    LISTING_DETAIL_RESULT_CHARGE_EVENT,
                );
                const saved = Math.min(chargeResult.chargedCount, 1);
                savedCount += saved;
                if (saved !== 1 || chargeResult.eventChargeLimitReached) {
                    statusMessage = `Charge limit reached after saving ${savedCount} Kleinanzeigen listing detail result(s).`;
                    break;
                }
            } catch (error) {
                const message = errorSummary(error);
                failures.push({ ad_id: listing.adId, error: message, outcome: 'failed' });
                console.warn(`Kleinanzeigen listing ${listing.adId} failed: ${message}`);
            }
        }
        await (await Actor.openKeyValueStore()).setValue('OUTPUT', {
            listings_requested: plan.listings.length,
            listings_completed: completedCount,
            listings_saved: savedCount,
            listings_failed: failures.length,
            status_message: statusMessage,
            failures,
        });
        if (statusMessage) { await Actor.exit({ statusMessage }); return; }
    } catch (error) {
        const rawMessage = errorSummary(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Kleinanzeigen details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }
    await Actor.exit();
}

main().catch((error) => { console.error('Actor failed: ' + errorSummary(error)); process.exitCode = 1; });
