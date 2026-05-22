import { Actor } from 'apify';

export const REDFIN_PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

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

export interface PushChargedPropertiesResult {
    savedCount: number;
    statusMessage: string | null;
}

export function getChargeLimitStatus(
    actor: ActorChargingApi,
    totalResults: number,
    searchIndex: number,
): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(REDFIN_PROPERTY_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching Redfin search ${searchIndex + 1}; ${totalResults} property result(s) were saved.`;
}

export async function pushChargedProperties(
    actor: ActorChargingApi,
    properties: Record<string, unknown>[],
    searchIndex: number,
): Promise<PushChargedPropertiesResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await actor.pushData(properties);
        return { savedCount: properties.length, statusMessage: null };
    }

    let savedCount = 0;
    for (const property of properties) {
        const chargeResult = await actor.pushData(property, REDFIN_PROPERTY_RESULT_CHARGE_EVENT);
        if (chargeResult.chargedCount >= 1) {
            savedCount += 1;
        }

        if (chargeResult.eventChargeLimitReached) {
            const statusMessage = chargeResult.chargedCount >= 1
                ? `Charge limit reached after saving ${savedCount} of ${properties.length} Redfin property result(s) for search ${searchIndex + 1}.`
                : `Charge limit reached before saving the next Redfin property result for search ${searchIndex + 1}.`;
            console.log(statusMessage, JSON.stringify({
                event: REDFIN_PROPERTY_RESULT_CHARGE_EVENT,
                charged_count: chargeResult.chargedCount,
                saved_count: savedCount,
                requested_count: properties.length,
                search_index: searchIndex,
            }));
            return { savedCount, statusMessage };
        }
    }

    return { savedCount, statusMessage: null };
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
