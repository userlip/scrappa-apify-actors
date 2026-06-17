export interface ChargeResultLike {
    chargedCount: number;
}

export function getSavedCount(chargeResult: ChargeResultLike, requestedCount: number): number {
    return Math.min(chargeResult.chargedCount, requestedCount);
}
