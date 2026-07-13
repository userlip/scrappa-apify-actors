export const VINTED_USER_PROFILE_RESULT_CHARGE_EVENT = 'user-profile-result';

interface ChargeResult {
    eventChargeLimitReached?: boolean;
    chargedCount?: number;
}

interface ChargingManager {
    getPricingInfo(): {
        isPayPerEvent?: boolean;
    };
    calculateMaxEventChargeCountWithinLimit(eventName: string): number;
}

interface ActorLike {
    getChargingManager(): ChargingManager;
    pushData(data: unknown, eventName?: string): Promise<ChargeResult | void>;
}

export function getVintedUserProfileAvailableChargeCount(
    actor: Pick<ActorLike, 'getChargingManager'>,
): number | null {
    const chargingManager = actor.getChargingManager();
    if (!chargingManager.getPricingInfo().isPayPerEvent) {
        return null;
    }

    const available = chargingManager.calculateMaxEventChargeCountWithinLimit(VINTED_USER_PROFILE_RESULT_CHARGE_EVENT);
    return Number.isFinite(available) ? Math.max(0, Math.floor(available)) : 0;
}

export interface PushVintedUserProfileResult {
    saved: boolean;
    statusMessage: string | null;
    chargedCount: number;
    eventChargeLimitReached: boolean;
}

export function getVintedUserProfileChargeLimitStatus(
    actor: Pick<ActorLike, 'getChargingManager'>,
    savedProfiles: number,
    requestIndex: number,
): string | null {
    const available = getVintedUserProfileAvailableChargeCount(actor);
    if (available === null) {
        return null;
    }

    if (available > 0) {
        return null;
    }

    return `Charge limit reached before fetching Vinted user profile request ${requestIndex + 1}; ${savedProfiles} profile result(s) were saved.`;
}

export async function pushSuccessfulVintedUserProfile(
    actor: ActorLike,
    item: Record<string, unknown>,
    requestIndex: number,
): Promise<PushVintedUserProfileResult> {
    const isPayPerEvent = actor.getChargingManager().getPricingInfo().isPayPerEvent === true;

    if (!isPayPerEvent) {
        await actor.pushData(item);
        return {
            saved: true,
            statusMessage: null,
            chargedCount: 1,
            eventChargeLimitReached: false,
        };
    }

    const chargeResult = await actor.pushData(item, VINTED_USER_PROFILE_RESULT_CHARGE_EVENT);
    const chargedCount = chargeResult?.chargedCount ?? 0;
    const eventChargeLimitReached = chargeResult?.eventChargeLimitReached === true;
    const saved = chargedCount >= 1;
    const statusMessage = eventChargeLimitReached
        ? saved
            ? `Charge limit reached after saving Vinted user profile result ${requestIndex + 1}.`
            : `Charge limit reached before saving Vinted user profile result ${requestIndex + 1}.`
        : null;

    return {
        saved,
        statusMessage,
        chargedCount,
        eventChargeLimitReached,
    };
}
