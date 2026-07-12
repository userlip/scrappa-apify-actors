export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    timeoutMs?: number;
}

interface ScrappaRequestOptions {
    attempts?: number;
}

interface ScrappaError {
    error?: string;
    message?: string;
    errors?: Record<string, string[]>;
    retryable?: boolean;
}

interface ScrappaErrorDetails {
    message: string;
    retryable: boolean;
}

const RETRYABLE_HTTP_STATUSES = new Set([408, 429, 500, 502, 503, 504]);

export class ScrappaHttpError extends Error {
    constructor(
        public readonly status: number,
        message: string,
        public readonly retryable = false,
    ) {
        super(`Scrappa API error (${status}): ${message}`);
        this.name = 'ScrappaHttpError';
    }
}

export class ScrappaConnectionError extends Error {
    constructor(options?: ErrorOptions) {
        super('Scrappa API connection failed', options);
        this.name = 'ScrappaConnectionError';
    }
}

export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) {
        super(`Scrappa API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaTimeoutError';
    }
}

export function getRetryDelayMs(failedAttempt: number, jitterMs = Math.random() * 1000): number {
    return Math.min(1000 * Math.pow(2, failedAttempt) + jitterMs, 10000);
}

export function isRetryableScrappaError(error: unknown): boolean {
    return error instanceof ScrappaTimeoutError
        || error instanceof ScrappaConnectionError
        || (error instanceof ScrappaHttpError
            && (RETRYABLE_HTTP_STATUSES.has(error.status)
                || (error.status === 403 && error.retryable)));
}

export class ScrappaClient {
    private apiKey: string;
    private baseUrl: string;
    private timeoutMs: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.timeoutMs = config.timeoutMs ?? 60000;
    }

    async get<T>(
        endpoint: string,
        params: Record<string, unknown> = {},
        options: ScrappaRequestOptions = {},
    ): Promise<T> {
        const attempts = Math.max(1, options.attempts ?? 1);
        let lastError: unknown;

        for (let attempt = 1; attempt <= attempts; attempt++) {
            try {
                return await this.send<T>(endpoint, params);
            } catch (error) {
                lastError = error;
                if (attempt >= attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const delayMs = getRetryDelayMs(attempt);
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`Scrappa API request failed (${message}). Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(endpoint: string, params: Record<string, unknown>): Promise<T> {
        const baseUrl = this.baseUrl.replace(/\/+$/, '');
        const endpointPath = endpoint.replace(/^\/+/, '');
        const url = new URL(`${baseUrl}/${endpointPath}`);
        for (const [key, value] of Object.entries(params)) {
            if (value !== undefined && value !== null && value !== '') {
                if (typeof value === 'boolean') {
                    if (value) url.searchParams.set(key, '1');
                } else {
                    url.searchParams.set(key, String(value));
                }
            }
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            const response = await this.fetchResponse(url, controller.signal);

            if (!response.ok) {
                const error = await this.readError(response);
                throw new ScrappaHttpError(response.status, error.message, error.retryable);
            }

            return await response.json() as T;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new ScrappaTimeoutError(this.timeoutMs, { cause: error });
            }
            throw error;
        } finally {
            clearTimeout(timeoutId);
        }
    }

    private async fetchResponse(url: URL, signal: AbortSignal): Promise<Response> {
        try {
            return await fetch(url, {
                headers: {
                    'X-API-Key': this.apiKey,
                    Accept: 'application/json',
                    'User-Agent': 'thescrappa-google-hotels-autocomplete-scraper/1.0',
                },
                signal,
            });
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw error;
            }

            throw new ScrappaConnectionError({ cause: error });
        }
    }

    private async readError(response: Response): Promise<ScrappaErrorDetails> {
        const fallback = response.statusText || `HTTP ${response.status}`;
        let bodyText: string;
        try {
            bodyText = await response.text();
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') throw error;
            return { message: fallback, retryable: false };
        }

        if (!bodyText) return { message: fallback, retryable: false };

        try {
            const errorData = JSON.parse(bodyText) as ScrappaError;
            let message = errorData.message ?? errorData.error ?? fallback;
            if (errorData.errors) {
                const details = Object.entries(errorData.errors)
                    .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                    .join('; ');
                if (details) message += ` - ${details}`;
            }
            return { message, retryable: errorData.retryable === true };
        } catch {
            return {
                message: bodyText.replace(/\s+/g, ' ').trim().slice(0, 500),
                retryable: false,
            };
        }
    }
}
