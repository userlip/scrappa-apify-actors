import { ScrappaTimeoutError } from './shared/scrappa-client.js';

export function formatGooglePatentsDetailsError(error: unknown, timeoutMs: number): unknown {
    if (error instanceof ScrappaTimeoutError) {
        return `${error.message}. The Google Patents details request exceeded the ${timeoutMs / 1000}s Scrappa API timeout. Run the request again or try a smaller batch.`;
    }

    return error;
}
