import { ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

export function isTransientRedfinValuationError(
    error: unknown,
): error is ScrappaHttpError | ScrappaTimeoutError {
    if (error instanceof ScrappaTimeoutError) {
        return true;
    }

    return error instanceof ScrappaHttpError && error.status >= 500 && error.status <= 599;
}

export function getTransientRedfinValuationStatus(error: ScrappaHttpError | ScrappaTimeoutError): string {
    const reason = error instanceof ScrappaHttpError
        ? `Scrappa upstream returned ${error.status} after retries`
        : error.message;

    return `${reason}; no Redfin valuation result was written or charged. Try the run again later.`;
}
