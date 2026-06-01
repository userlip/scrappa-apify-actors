import { Actor } from 'apify';
import type { DomainAvailabilityDatasetItem } from './results.js';

export const DOMAIN_RESULT_CHARGE_EVENT = 'domain-result';

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

export interface PushDomainResult {
    saved: boolean;
    statusMessage: string | null;
}

export function getDomainChargeLimitStatus(
    actor: ActorChargingApi,
    processed: number,
    requested: number,
): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(DOMAIN_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching the next domain availability result; ${processed} of ${requested} domain(s) were processed.`;
}

export async function pushDomainResult(
    actor: ActorChargingApi,
    item: DomainAvailabilityDatasetItem,
): Promise<PushDomainResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent || !item.success) {
        await actor.pushData(item);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(item, DOMAIN_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
        const statusMessage = `Charge limit reached before saving ${item.domain}; stopping batch without writing uncharged success results.`;
        console.warn(statusMessage, JSON.stringify({
            event: DOMAIN_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
        }));
        return { saved: false, statusMessage };
    }

    return { saved: true, statusMessage: null };
}

export const actorChargingApi = Actor as unknown as ActorChargingApi;
