export const JAMEDA_REVIEW_RESULT_CHARGE_EVENT = 'jameda-review-result';

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

function assertChargeResult(value: ChargeResult | void): asserts value is ChargeResult {
    if (!value || typeof value.chargedCount !== 'number') {
        throw new Error('Apify pushData did not return a chargedCount for the Jameda review charge event.');
    }
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

    const chargeResult = await dataset.pushData(items, JAMEDA_REVIEW_RESULT_CHARGE_EVENT);
    assertChargeResult(chargeResult);

    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Jameda review results.`;
        console.log(statusMessage, JSON.stringify({
            event: JAMEDA_REVIEW_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}
