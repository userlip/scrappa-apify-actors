export function isActorLevelScrappaFailure(error: unknown): boolean {
    if (!(error instanceof Error)) {
        return false;
    }

    return /Scrappa API error \((?:401|403)\)/.test(error.message);
}
