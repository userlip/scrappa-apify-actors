import {
    REQUEST_ATTEMPTS,
    REQUEST_TIMEOUT_MS,
    RETRY_BACKOFF_MS,
} from '../runtime-config.js';

export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) {
        super(`Scrappa API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaTimeoutError';
    }
}

export class ScrappaHttpError extends Error {
    constructor(readonly status: number) {
        super(`Scrappa API error (${status})`);
        this.name = 'ScrappaHttpError';
    }
}

export function isRetryableScrappaError(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError) {
        return true;
    }

    if (error instanceof ScrappaHttpError) {
        return [408, 429, 500, 502, 503, 504].includes(error.status);
    }

    return error instanceof TypeError
        && /fetch failed|failed to fetch|network|terminated|reset|econnrefused|econnreset|socket hang up|chunk/i.test(error.message);
}

export class ScrappaClient {
    constructor(private readonly config: { apiKey: string; baseUrl?: string; timeoutMs?: number }) {
    }

    async get<T>(endpoint: string, params: object, attempts = REQUEST_ATTEMPTS): Promise<T> {
        let lastError: unknown;

        for (let attempt = 1; attempt <= attempts; attempt += 1) {
            try {
                return await this.getOnce<T>(endpoint, params);
            } catch (error) {
                lastError = error;

                if (attempt === attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                await new Promise((resolve) => setTimeout(resolve, attempt * RETRY_BACKOFF_MS));
            }
        }

        throw lastError instanceof Error
            ? lastError
            : new Error('Scrappa API request failed');
    }

    private async getOnce<T>(endpoint: string, params: object): Promise<T> {
        const baseUrl = this.config.baseUrl ?? 'https://scrappa.co/api';
        const url = new URL(`${baseUrl}${endpoint}`);

        for (const [key, value] of Object.entries(params)) {
            if (value !== null && value !== '') {
                url.searchParams.set(key, String(value));
            }
        }

        const timeoutMs = this.config.timeoutMs ?? REQUEST_TIMEOUT_MS;
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), timeoutMs);

        try {
            const response = await fetch(url, {
                headers: {
                    'X-API-Key': this.config.apiKey,
                    Accept: 'application/json',
                },
                signal: controller.signal,
            });

            if (!response.ok) {
                throw new ScrappaHttpError(response.status);
            }

            return await response.json() as T;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new ScrappaTimeoutError(timeoutMs, { cause: error });
            }

            throw error;
        } finally {
            clearTimeout(timer);
        }
    }
}
