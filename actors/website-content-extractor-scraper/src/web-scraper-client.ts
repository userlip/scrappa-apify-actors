import type { WebScraperParams } from './request-params.js';

export interface ScrappaWebScraperClientConfig {
    apiKey: string;
    baseUrl?: string;
    timeoutMs?: number;
    maxAttempts?: number;
    retryDelayMs?: number;
}

export interface ScrappaWebScraperErrorDetails {
    message?: string;
    error?: string;
    error_code?: string;
    errors?: Record<string, string[]>;
    diagnostics?: unknown;
}

export class ScrappaWebScraperHttpError extends Error {
    constructor(
        public readonly status: number,
        public readonly details: string,
        public readonly body: ScrappaWebScraperErrorDetails | null = null,
    ) {
        super(`Scrappa Web Scraper API error (${status}): ${details}`);
        this.name = 'ScrappaWebScraperHttpError';
    }
}

export class ScrappaWebScraperTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) {
        super(`Scrappa Web Scraper API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaWebScraperTimeoutError';
    }
}

export class ScrappaWebScraperClient {
    private readonly apiKey: string;
    private readonly baseUrl: string;
    private readonly timeoutMs: number;
    private readonly maxAttempts: number;
    private readonly retryDelayMs: number;

    constructor(config: ScrappaWebScraperClientConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.timeoutMs = config.timeoutMs ?? 90000;
        this.maxAttempts = Math.max(1, config.maxAttempts ?? 2);
        this.retryDelayMs = Math.max(0, config.retryDelayMs ?? 1000);
    }

    async scrapeJson<T>(params: WebScraperParams): Promise<T> {
        const response = await this.fetchResponse(params, 'application/json');
        return await response.json() as T;
    }

    async scrapeMarkdown(params: WebScraperParams): Promise<string> {
        const response = await this.fetchResponse(params, 'text/markdown, text/plain;q=0.9, */*;q=0.8');
        return await response.text();
    }

    private async fetchResponse(params: WebScraperParams, accept: string): Promise<Response> {
        let lastError: unknown;

        for (let attempt = 1; attempt <= this.maxAttempts; attempt += 1) {
            try {
                return await this.sendOnce(params, accept);
            } catch (error) {
                lastError = error;

                if (attempt >= this.maxAttempts || !isRetryableError(error)) {
                    break;
                }

                console.warn(`Scrappa Web Scraper API request failed (${describeError(error)}). Retrying attempt ${attempt + 1}/${this.maxAttempts}.`);
                await delay(this.retryDelayMs);
            }
        }

        throw lastError;
    }

    private async sendOnce(params: WebScraperParams, accept: string): Promise<Response> {
        const url = new URL(`${this.baseUrl}/web-scraper`);
        Object.entries(params).forEach(([key, value]) => {
            if (value === undefined || value === null || value === '') {
                return;
            }

            if (typeof value === 'boolean') {
                if (value) {
                    url.searchParams.set(key, '1');
                }
                return;
            }

            url.searchParams.set(key, String(value));
        });

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            const response = await fetch(url.toString(), {
                method: 'GET',
                headers: {
                    'X-API-Key': this.apiKey,
                    'Accept': accept,
                    'User-Agent': 'thescrappa-website-content-extractor-scraper/1.0',
                },
                signal: controller.signal,
            });

            if (!response.ok) {
                throw await this.buildHttpError(response);
            }

            return response;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new ScrappaWebScraperTimeoutError(this.timeoutMs, { cause: error });
            }

            throw error;
        } finally {
            clearTimeout(timeoutId);
        }
    }

    private async buildHttpError(response: Response): Promise<ScrappaWebScraperHttpError> {
        const fallback = response.statusText || `HTTP ${response.status}`;
        const contentType = response.headers.get('content-type') ?? '';
        const bodyText = await response.text();

        if (!bodyText) {
            return new ScrappaWebScraperHttpError(response.status, fallback);
        }

        if (contentType.includes('application/json')) {
            try {
                const body = JSON.parse(bodyText) as ScrappaWebScraperErrorDetails;
                return new ScrappaWebScraperHttpError(response.status, describeJsonError(body, fallback), body);
            } catch {
                return new ScrappaWebScraperHttpError(response.status, cleanText(bodyText));
            }
        }

        return new ScrappaWebScraperHttpError(response.status, cleanText(bodyText));
    }
}

function isRetryableError(error: unknown): boolean {
    if (error instanceof ScrappaWebScraperTimeoutError) {
        return true;
    }

    if (error instanceof ScrappaWebScraperHttpError) {
        return [429, 500, 502, 503, 504].includes(error.status);
    }

    return error instanceof TypeError;
}

function describeJsonError(body: ScrappaWebScraperErrorDetails, fallback: string): string {
    let message = body.message ?? body.error ?? fallback;
    if (body.errors) {
        const errorDetails = Object.entries(body.errors)
            .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
            .join('; ');
        if (errorDetails) {
            message += ` - ${errorDetails}`;
        }
    }

    return message;
}

function cleanText(value: string): string {
    return value.replace(/\s+/g, ' ').trim().slice(0, 500);
}

function describeError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    return String(error);
}

async function delay(ms: number): Promise<void> {
    if (ms === 0) {
        return;
    }

    await new Promise((resolve) => setTimeout(resolve, ms));
}
