export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    timeoutMs?: number;
}

interface ScrappaRequestOptions {
    attempts?: number;
}

interface ScrappaError {
    message?: string;
    error?: string;
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

export class ScrappaNetworkError extends Error {
    constructor(message: string, options?: ErrorOptions) {
        super(`Scrappa API network error: ${message}`, options);
        this.name = 'ScrappaNetworkError';
    }
}

export class ScrappaHttpError extends Error {
    status: number;
    details: string;

    constructor(status: number, details: string) {
        super(`Scrappa API error (${status}): ${details}`);
        this.name = 'ScrappaHttpError';
        this.status = status;
        this.details = details;
    }
}

export function getRetryDelayMs(failedAttempt: number, jitterMs = Math.random() * 1000): number {
    return Math.min(1000 * Math.pow(2, failedAttempt) + jitterMs, 10000);
}

export function isRetryableScrappaError(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError || error instanceof ScrappaNetworkError) {
        return true;
    }

    if (!(error instanceof Error)) {
        return false;
    }

    if (error instanceof ScrappaHttpError) {
        return [408, 429, 500, 502, 503, 504].includes(error.status);
    }

    return false;
}

export function describeScrappaError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    return String(error);
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
                return await this.send<T>(endpoint, params);
            } catch (error) {
                lastError = error;

                if (attempt >= attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const delayMs = getRetryDelayMs(attempt);
                console.warn(`Scrappa API request failed (${describeScrappaError(error)}). Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(endpoint: string, params: Record<string, unknown> = {}): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);

        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== null && value !== '') {
                url.searchParams.set(key, String(value));
            }
        });

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
            'User-Agent': 'thescrappa-google-translate-scraper/1.0',
        };

        if (this.debug) {
            console.log(`[Scrappa] GET ${url.toString()}`);
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            let response: Response;
            try {
                response = await fetch(url.toString(), {
                    method: 'GET',
                    headers,
                    signal: controller.signal,
                });
            } catch (error) {
                if (error instanceof Error && error.name === 'AbortError') {
                    throw new ScrappaTimeoutError(this.timeoutMs, { cause: error });
                }

                if (error instanceof TypeError) {
                    throw new ScrappaNetworkError(error.message, { cause: error });
                }

                throw error;
            }

            if (!response.ok) {
                const errorMessage = await this.readErrorMessage(response);
                throw new ScrappaHttpError(response.status, errorMessage);
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
            let message = errorData.message ?? errorData.error ?? fallback;
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
