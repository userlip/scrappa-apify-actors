export const DOCTOR_PROFILE_RESULT_CHARGE_EVENT = 'doctor-profile-result';

export interface PushChargedItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

export interface ChargeResult {
    eventChargeLimitReached?: boolean;
    chargedCount: number;
}

export interface ChargedDataset {
    isPayPerEvent(): boolean;
    pushData(items: Record<string, unknown>[], eventName?: string): Promise<ChargeResult | void>;
}

export async function pushChargedItems(
    dataset: ChargedDataset,
    items: Record<string, unknown>[],
): Promise<PushChargedItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    if (!dataset.isPayPerEvent()) {
        await dataset.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await dataset.pushData(items, DOCTOR_PROFILE_RESULT_CHARGE_EVENT) as ChargeResult;
    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Jameda doctor profile results.`;
        console.log(statusMessage, JSON.stringify({
            event: DOCTOR_PROFILE_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}
