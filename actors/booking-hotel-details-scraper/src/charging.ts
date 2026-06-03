export const BOOKING_HOTEL_RESULT_CHARGE_EVENT = 'hotel-result';

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
    pushData(data: unknown, eventName?: string): Promise<ChargeResult>;
}

export interface PushHotelItemResult {
    saved: boolean;
    statusMessage: string | null;
    chargedCount: number;
    eventChargeLimitReached: boolean;
}

export function getHotelChargeLimitStatus(actor: Pick<ActorLike, 'getChargingManager'>, savedResults: number, requestIndex: number): string | null {
    const chargingManager = actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();

    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(BOOKING_HOTEL_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching Booking.com hotel detail request ${requestIndex + 1}; ${savedResults} hotel detail result(s) were saved.`;
}

export async function pushSuccessfulHotelItem(actor: ActorLike, item: Record<string, unknown>, requestIndex: number): Promise<PushHotelItemResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();

    if (isPayPerEvent) {
        const chargeResult = await actor.pushData(item, BOOKING_HOTEL_RESULT_CHARGE_EVENT);
        const chargedCount = chargeResult.chargedCount ?? 0;
        const eventChargeLimitReached = Boolean(chargeResult.eventChargeLimitReached);
        const saved = chargedCount >= 1;
        const statusMessage = eventChargeLimitReached
            ? saved
                ? `Charge limit reached after saving Booking.com hotel detail result ${requestIndex + 1}.`
                : `Charge limit reached before saving Booking.com hotel detail result ${requestIndex + 1}.`
            : null;

        return {
            saved,
            statusMessage,
            chargedCount,
            eventChargeLimitReached,
        };
    }

    await actor.pushData(item);

    return {
        saved: true,
        statusMessage: null,
        chargedCount: 1,
        eventChargeLimitReached: false,
    };
}

export async function pushErrorHotelItem(actor: Pick<ActorLike, 'pushData'>, item: Record<string, unknown>): Promise<void> {
    await actor.pushData(item);
}
