export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    timeoutMs?: number;
}

interface ScrappaRequestOptions {
    attempts?: number;
    retryDelayMs?: (failedAttempt: number) => number;
}

export interface ScrappaError {
    status: number;
    message: string;
    errors?: Record<string, string[]>;
}

export class ScrappaHttpError extends Error {
    constructor(public readonly status: number, details: string) {
        super(`Scrappa API error (${status}): ${details}`);
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

const RETRYABLE_HTTP_STATUSES = new Set([408, 429, 500, 502, 503, 504]);

export function isRetryableScrappaError(error: unknown): boolean {
    return error instanceof ScrappaTimeoutError
        || error instanceof ScrappaConnectionError
        || (error instanceof ScrappaHttpError && RETRYABLE_HTTP_STATUSES.has(error.status));
}

export function getRetryDelayMs(failedAttempt: number, jitterMs = Math.random() * 1000): number {
    return Math.min(1000 * Math.pow(2, failedAttempt) + jitterMs, 10000);
}

export class ScrappaClient {
    private apiKey: string;
    private baseUrl: string;
    private debug: boolean;
    private timeoutMs: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
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
                return await this.request<T>('GET', endpoint, params);
            } catch (error) {
                lastError = error;
                if (attempt >= attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const delayMs = options.retryDelayMs?.(attempt) ?? getRetryDelayMs(attempt);
                const message = error instanceof Error ? error.message : String(error);
                console.warn(`Scrappa API request failed (${message}). Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    async post<T>(endpoint: string, body: Record<string, unknown> = {}): Promise<T> {
        return this.request<T>('POST', endpoint, {}, body);
    }

    private async request<T>(
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
                    url.searchParams.set(key, String(value));
                }
            });
        }

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
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
            let response: Response;
            try {
                response = await fetch(url.toString(), {
                    ...options,
                    signal: controller.signal,
                });
            } catch (error) {
                if (error instanceof Error && error.name === 'AbortError') {
                    throw new ScrappaTimeoutError(this.timeoutMs, { cause: error });
                }

                throw new ScrappaConnectionError({ cause: error });
            }

            if (!response.ok) {
                let errorMessage: string;
                const responseClone = response.clone();

                try {
                    const errorData = await response.json() as { message?: string; errors?: Record<string, string[]> };
                    errorMessage = errorData.message ?? `HTTP ${response.status}`;

                    if (errorData.errors) {
                        const details = Object.entries(errorData.errors)
                            .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                            .join('; ');
                        errorMessage += ` - ${details}`;
                    }
                } catch (parseError) {
                    if (parseError instanceof Error && parseError.name === 'AbortError') {
                        throw parseError;
                    }
                    errorMessage = await responseClone.text() || `HTTP ${response.status}`;
                }

                throw new ScrappaHttpError(response.status, errorMessage);
            }

            return response.json() as Promise<T>;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new ScrappaTimeoutError(this.timeoutMs, { cause: error });
            }
            throw error;
        } finally {
            clearTimeout(timeoutId);
        }
    }
}
