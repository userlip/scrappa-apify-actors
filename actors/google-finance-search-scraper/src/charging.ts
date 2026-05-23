import { Actor } from 'apify';

export const FINANCE_SEARCH_RESULT_CHARGE_EVENT = 'finance-search-result';

interface ChargingManager {
    getPricingInfo(): { isPayPerEvent: boolean };
}

interface ActorChargingApi {
    getChargingManager(): ChargingManager;
    pushData(data: Record<string, unknown>[], eventName?: string): Promise<{
        chargedCount: number;
        eventChargeLimitReached: boolean;
    }>;
}

export interface PushSearchItemsResult {
    pushed: boolean;
    statusMessage: string | null;
}

export async function pushSearchItems(
    actor: ActorChargingApi,
    items: Record<string, unknown>[],
): Promise<PushSearchItemsResult> {
    if (items.length === 0) {
        return { pushed: true, statusMessage: null };
    }

    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await actor.pushData(items);
        return { pushed: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(items, FINANCE_SEARCH_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < items.length) {
        return {
            pushed: false,
            statusMessage: 'Charge limit reached before saving all Google Finance search results.',
        };
    }

    return { pushed: true, statusMessage: null };
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
