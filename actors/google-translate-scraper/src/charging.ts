import { Actor } from 'apify';
import type { TranslationDatasetItem } from './results.js';

export const TRANSLATION_RESULT_CHARGE_EVENT = 'translation-result';

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

export interface PushTranslationResult {
    saved: boolean;
    statusMessage: string | null;
}

export function getTranslationChargeLimitStatus(
    actor: ActorChargingApi,
    processed: number,
    requested: number,
): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(TRANSLATION_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching the next translation; ${processed} of ${requested} translation item(s) were processed.`;
}

export async function pushTranslationResult(
    actor: ActorChargingApi,
    item: TranslationDatasetItem,
): Promise<PushTranslationResult> {
    const datasetItem = { ...item };
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent || !item.success) {
        await actor.pushData(datasetItem);
        return { saved: true, statusMessage: null };
    }

    const chargeResult = await actor.pushData(datasetItem, TRANSLATION_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
        const statusMessage = `Charge limit reached before saving translation ${item.index + 1}; stopping batch without writing uncharged success results.`;
        console.warn(statusMessage, JSON.stringify({
            event: TRANSLATION_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
        }));
        return { saved: false, statusMessage };
    }

    return { saved: true, statusMessage: null };
}

// Apify SDK 3.x exposes these charging methods at runtime, but its public type
// surface does not model the pay-per-event helpers narrowly enough for tests.
export const actorChargingApi = Actor as unknown as ActorChargingApi;
