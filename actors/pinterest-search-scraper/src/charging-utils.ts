export interface PinterestChargeResultLike {
    chargedCount: number;
    eventChargeLimitReached?: boolean;
}

export interface PinterestChargedSaveResult {
    savedCount: number;
    chargeLimitReached: boolean;
}

export function getPinterestChargedSaveResult(
    chargeResult: PinterestChargeResultLike,
    requestedCount: number,
): PinterestChargedSaveResult {
    const savedCount = Math.min(chargeResult.chargedCount, requestedCount);

    return {
        savedCount,
        chargeLimitReached: Boolean(chargeResult.eventChargeLimitReached) || savedCount < requestedCount,
    };
}
