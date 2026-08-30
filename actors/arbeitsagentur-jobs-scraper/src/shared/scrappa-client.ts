export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    maxRetryDelayMs?: number;
    timeoutMs?: number;
}

interface ScrappaRequestOptions {
    attempts?: number;
}

interface ScrappaError {
    message?: string;
    errors?: Record<string, string[]>;
}

interface ScrappaTimeoutErrorOptions extends ErrorOptions {
    message?: string;
}

export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ScrappaTimeoutErrorOptions) {
        super(options?.message ?? `Scrappa API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaTimeoutError';
    }
}

export class ScrappaApiError extends Error {
    constructor(
        public readonly statusCode: number,
        message: string,
        public readonly retryAfterMs?: number,
    ) {
        super(`Scrappa API error (${statusCode}): ${message}`);
        this.name = 'ScrappaApiError';
    }
}

export function getRetryDelayMs(
    failedAttempt: number,
    jitterMs = Math.random() * 1000,
    retryAfterMs?: number,
    maxRetryDelayMs = 60000,
): number {
    const exponentialDelayMs = 1000 * Math.pow(2, failedAttempt) + jitterMs;
    return Math.min(Math.max(exponentialDelayMs, retryAfterMs ?? 0), maxRetryDelayMs);
}

export function isRetryableScrappaError(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError) {
        return true;
    }

    if (error instanceof ScrappaApiError) {
        return [408, 429, 500, 502, 503, 504].includes(error.statusCode);
    }

    return false;
}

export class ScrappaClient {
    private apiKey: string;
    private baseUrl: string;
    private debug: boolean;
    private maxRetryDelayMs: number;
    private timeoutMs: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
        this.maxRetryDelayMs = config.maxRetryDelayMs ?? 60000;
        this.timeoutMs = config.timeoutMs ?? 60000;
    }

    async get<T>(
        endpoint: string,
        params: Record<string, unknown> = {},
        options: ScrappaRequestOptions = {}
    ): Promise<T> {
        return this.request<T>('GET', endpoint, params, undefined, options);
    }

    async post<T>(
        endpoint: string,
        body: Record<string, unknown> = {},
        options: ScrappaRequestOptions = {}
    ): Promise<T> {
        return this.request<T>('POST', endpoint, {}, body, options);
    }

    private async request<T>(
        method: 'GET' | 'POST',
        endpoint: string,
        params: Record<string, unknown> = {},
        body?: Record<string, unknown>,
        options: ScrappaRequestOptions = {}
    ): Promise<T> {
        const attempts = Math.max(1, options.attempts ?? 1);
        let lastError: unknown;

        for (let attempt = 1; attempt <= attempts; attempt++) {
            try {
                return await this.send<T>(method, endpoint, params, body);
            } catch (error) {
                lastError = error;

                if (attempt >= attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const retryAfterMs = error instanceof ScrappaApiError ? error.retryAfterMs : undefined;
                const delayMs = getRetryDelayMs(attempt, undefined, retryAfterMs, this.maxRetryDelayMs);
                console.warn(`Scrappa API request failed (${this.describeError(error)}). Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(
        method: 'GET' | 'POST',
        endpoint: string,
        params: Record<string, unknown> = {},
        body?: Record<string, unknown>
    ): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);

        // Add query params for GET requests
        if (method === 'GET') {
            Object.entries(params).forEach(([key, value]) => {
                if (value !== undefined && value !== null && value !== '') {
                    if (typeof value === 'boolean') {
                        url.searchParams.set(key, value ? 'true' : 'false');
                    } else {
                        url.searchParams.set(key, String(value));
                    }
                }
            });
        }

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
            'User-Agent': 'thescrappa-arbeitsagentur-jobs-scraper/1.0',
        };

        const options: RequestInit = {
            method,
            headers,
        };

        if (method === 'POST' && body) {
            headers['Content-Type'] = 'application/json';
            options.body = JSON.stringify(body);
        }

        if (this.debug) {
            console.log(`[Scrappa] ${method} ${url.toString()}`);
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            const response = await fetch(url.toString(), {
                ...options,
                signal: controller.signal,
            });

            if (!response.ok) {
                const errorMessage = await this.readErrorMessage(response);

                const retryAfterMs = this.parseRetryAfterMs(response.headers.get('Retry-After'));
                throw new ScrappaApiError(response.status, errorMessage, retryAfterMs);
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

    private describeError(error: unknown): string {
        if (error instanceof Error) {
            return error.message;
        }

        return String(error);
    }

    private parseRetryAfterMs(retryAfter: string | null): number | undefined {
        if (retryAfter === null) {
            return undefined;
        }

        const seconds = Number(retryAfter);
        if (Number.isFinite(seconds) && seconds >= 0) {
            return seconds * 1000;
        }

        const retryAt = Date.parse(retryAfter);
        if (Number.isNaN(retryAt)) {
            return undefined;
        }

        return Math.max(0, retryAt - Date.now());
    }

    private async readErrorMessage(response: Response): Promise<string> {
        const fallback = response.statusText || `HTTP ${response.status}`;

        let bodyText: string;
        try {
            bodyText = await response.text();
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw error;
            }

            return fallback;
        }

        if (!bodyText) {
            return fallback;
        }

        const jsonMessage = this.tryParseJsonError(bodyText, fallback);
        if (jsonMessage) {
            return jsonMessage;
        }

        return bodyText.replace(/\s+/g, ' ').trim().slice(0, 500);
    }

    private tryParseJsonError(bodyText: string, fallback: string): string | null {
        try {
            const errorData = JSON.parse(bodyText) as ScrappaError;
            let message = errorData.message ?? fallback;
            if (errorData.errors) {
                const errorDetails = Object.entries(errorData.errors)
                    .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                    .join('; ');
                if (errorDetails) {
                    message += ` - ${errorDetails}`;
                }
            }
            return message;
        } catch {
            return null;
        }
    }
}
