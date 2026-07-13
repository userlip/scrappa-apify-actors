import { ScrappaAuthError } from './shared/scrappa-client.js';

export function isActorLevelScrappaFailure(error: unknown): boolean {
    return error instanceof ScrappaAuthError;
}
