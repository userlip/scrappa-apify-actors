import type { KleinanzeigenDetailsPlanItem } from './request-params.js';
import {
    buildKleinanzeigenDetailsDatasetItem,
    selectKleinanzeigenDetail,
} from './response-utils.js';
import type { KleinanzeigenDetailsResponse } from './response-utils.js';
import { errorSummary } from './shared/error-utils.js';

export const LISTING_DETAIL_RESULT_CHARGE_EVENT = 'listing-detail-result';

interface ChargeResult {
    chargedCount?: number;
    eventChargeLimitReached?: boolean;
}

interface ActorLike {
    getChargingManager(): {
        getPricingInfo(): { isPayPerEvent?: boolean };
        calculateMaxEventChargeCountWithinLimit(eventName: string): number;
    };
    pushData(data: Record<string, unknown>, eventName?: string): Promise<ChargeResult>;
}

export interface ListingDetailsProcessingResult {
    savedCount: number;
    completedCount: number;
    failures: Array<{ ad_id: string; error: string; outcome: 'failed' }>;
    statusMessage: string | null;
}

export function buildListingDetailsOutput(
    listingsRequested: number,
    result: ListingDetailsProcessingResult,
): {
    listings_requested: number;
    listings_completed: number;
    listings_saved: number;
    listings_failed: number;
    status_message: string | null;
    failures: ListingDetailsProcessingResult['failures'];
} {
    return {
        listings_requested: listingsRequested,
        listings_completed: result.completedCount,
        listings_saved: result.savedCount,
        listings_failed: result.failures.length,
        status_message: result.statusMessage,
        failures: result.failures,
    };
}

function getChargeableListingCapacity(actor: Pick<ActorLike, 'getChargingManager'>): number {
    const manager = actor.getChargingManager();
    return manager.getPricingInfo().isPayPerEvent
        ? manager.calculateMaxEventChargeCountWithinLimit(LISTING_DETAIL_RESULT_CHARGE_EVENT)
        : Infinity;
}

async function saveListingDetail(actor: ActorLike, item: Record<string, unknown>): Promise<{ saved: boolean; chargeLimitReached: boolean }> {
    if (!actor.getChargingManager().getPricingInfo().isPayPerEvent) {
        await actor.pushData(item);
        return { saved: true, chargeLimitReached: false };
    }

    const chargeResult = await actor.pushData(item, LISTING_DETAIL_RESULT_CHARGE_EVENT);
    return {
        saved: Math.min(chargeResult.chargedCount ?? 0, 1) === 1,
        chargeLimitReached: Boolean(chargeResult.eventChargeLimitReached),
    };
}

export async function processKleinanzeigenListingDetails(
    actor: ActorLike,
    listings: KleinanzeigenDetailsPlanItem[],
    fetchListing: (adId: string) => Promise<KleinanzeigenDetailsResponse>,
): Promise<ListingDetailsProcessingResult> {
    const failures: ListingDetailsProcessingResult['failures'] = [];
    let savedCount = 0;
    let completedCount = 0;
    let statusMessage: string | null = null;

    for (const listing of listings) {
        if (getChargeableListingCapacity(actor) <= 0) {
            statusMessage = `Charge limit reached before fetching Kleinanzeigen listing ${listing.adId}.`;
            console.log(statusMessage);
            break;
        }
        completedCount++;
        try {
            const detail = selectKleinanzeigenDetail(await fetchListing(listing.adId));
            if (!detail) throw new Error('Scrappa response did not contain a recognizable listing detail payload');
            const saveResult = await saveListingDetail(
                actor,
                buildKleinanzeigenDetailsDatasetItem(detail, listing.adId, listing.index),
            );
            if (saveResult.saved) savedCount++;
            if (!saveResult.saved || saveResult.chargeLimitReached) {
                statusMessage = `Charge limit reached after saving ${savedCount} Kleinanzeigen listing detail result(s).`;
                break;
            }
        } catch (error) {
            const message = errorSummary(error);
            failures.push({ ad_id: listing.adId, error: message, outcome: 'failed' });
            console.warn(`Kleinanzeigen listing ${listing.adId} failed: ${message}`);
        }
    }

    return { savedCount, completedCount, failures, statusMessage };
}
