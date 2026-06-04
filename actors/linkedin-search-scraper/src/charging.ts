import { Actor } from 'apify';
import type { LinkedInSearchResult } from './search-response.js';

export const LINKEDIN_SEARCH_RESULT_CHARGE_EVENT = 'linkedin-search-result';

interface ChargingManager {
    getPricingInfo(): { isPayPerEvent: boolean };
    calculateMaxEventChargeCountWithinLimit(eventName: string): number;
}

interface ActorChargingApi {
    getChargingManager(): ChargingManager;
    pushData(data: Record<string, unknown>, eventName?: string): Promise<{
        chargedCount: number;
        eventChargeLimitReached: boolean;
    }>;
}

export interface PushLinkedInSearchResult {
    saved: boolean;
    statusMessage: string | null;
}

export function getRemainingLinkedInSearchResultCharges(actor: ActorChargingApi): number | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    return chargingManager.calculateMaxEventChargeCountWithinLimit(LINKEDIN_SEARCH_RESULT_CHARGE_EVENT);
}

export function getLinkedInSearchChargeLimitStatus(
    actor: ActorChargingApi,
    processed: number,
    requested: number,
): string | null {
    const remainingCharges = getRemainingLinkedInSearchResultCharges(actor);
    if (remainingCharges === null || remainingCharges > 0) {
        return null;
    }

    return `Charge limit reached before saving the next LinkedIn search result; ${processed} of ${requested} result(s) were saved.`;
}

export async function pushLinkedInSearchResult(
    actor: ActorChargingApi,
    result: LinkedInSearchResult,
): Promise<PushLinkedInSearchResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await actor.pushData(result);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(result, LINKEDIN_SEARCH_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
        const statusMessage = 'Charge limit reached before saving the LinkedIn search result; stopping without writing uncharged results.';
        console.warn(statusMessage, JSON.stringify({
            event: LINKEDIN_SEARCH_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
        }));
        return { saved: false, statusMessage };
    }

    return { saved: true, statusMessage: null };
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
