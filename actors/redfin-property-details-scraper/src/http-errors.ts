import { ScrappaHttpError } from './shared/index.js';

const PER_PROPERTY_HTTP_STATUSES = new Set([400, 404, 422]);
const REDFIN_PROPERTY_DETAILS_TERMINAL_ERROR = 'Failed to fetch property details after multiple attempts';

export function isPerPropertyScrappaHttpError(error: unknown): error is ScrappaHttpError {
    if (!(error instanceof ScrappaHttpError)) {
        return false;
    }

    return PER_PROPERTY_HTTP_STATUSES.has(error.status)
        || isTerminalRedfinPropertyDetailsError(error);
}

function isTerminalRedfinPropertyDetailsError(error: ScrappaHttpError): boolean {
    return error.status === 500
        && normalizeErrorDetails(error.details) === REDFIN_PROPERTY_DETAILS_TERMINAL_ERROR;
}

function normalizeErrorDetails(details: string): string {
    return details.trim().replace(/\.$/, '');
}
