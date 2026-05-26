import { Actor } from 'apify';

export const REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT = 'property-result';

interface ChargingManager {
    getPricingInfo(): { isPayPerEvent: boolean };
    calculateMaxEventChargeCountWithinLimit(eventName: string): number;
}

interface ActorChargingApi {
    getChargingManager(): ChargingManager;
    pushData(data: Record<string, unknown> | Record<string, unknown>[], eventName?: string): Promise<{
        chargedCount: number;
        eventChargeLimitReached: boolean;
    }>;
}

export interface PushChargedPropertyResult {
    saved: boolean;
    statusMessage: string | null;
}

export function getChargeLimitStatus(
    actor: ActorChargingApi,
    totalResults: number,
    propertyIndex: number,
): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching Redfin property ${propertyIndex + 1}; ${totalResults} property detail result(s) were saved.`;
}

export async function pushChargedProperty(
    actor: ActorChargingApi,
    property: Record<string, unknown>,
    propertyIndex: number,
): Promise<PushChargedPropertyResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await actor.pushData(property);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(property, REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        const statusMessage = chargeResult.chargedCount >= 1
            ? `Charge limit reached after saving Redfin property detail result ${propertyIndex + 1}.`
            : `Charge limit reached before saving Redfin property detail result ${propertyIndex + 1}.`;
        console.log(statusMessage, JSON.stringify({
            event: REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            property_index: propertyIndex,
        }));
        return { saved: chargeResult.chargedCount >= 1, statusMessage };
    }

    return { saved: chargeResult.chargedCount >= 1, statusMessage: null };
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
