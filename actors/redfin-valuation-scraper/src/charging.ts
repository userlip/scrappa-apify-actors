import { Actor } from 'apify';

export const REDFIN_VALUATION_RESULT_CHARGE_EVENT = 'valuation-result';

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

export async function pushChargedValuation(
    actor: ActorChargingApi,
    item: Record<string, unknown>,
): Promise<{ saved: boolean; statusMessage: string | null }> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await actor.pushData(item);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(item, REDFIN_VALUATION_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
        return {
            saved: false,
            statusMessage: 'Charge limit reached before saving the next Redfin valuation result.',
        };
    }

    return { saved: chargeResult.chargedCount >= 1, statusMessage: null };
}

export function getChargeLimitStatus(actor: ActorChargingApi, totalResults: number, requestIndex: number): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(REDFIN_VALUATION_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching Redfin valuation ${requestIndex + 1}; ${totalResults} valuation result(s) were saved.`;
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
