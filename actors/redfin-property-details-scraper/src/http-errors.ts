import { ScrappaHttpError } from './shared/index.js';

const PER_PROPERTY_HTTP_STATUSES = new Set([400, 404, 422]);

export function isPerPropertyScrappaHttpError(error: unknown): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError && PER_PROPERTY_HTTP_STATUSES.has(error.status);
}
