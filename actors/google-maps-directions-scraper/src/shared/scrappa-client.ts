export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    timeoutMs?: number;
    retryDelayMs?: number;
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
    if (error instanceof ScrappaTimeoutError) {
        return true;
    }
    if (!(error instanceof Error)) {
        return false;
    }
    if (/Scrappa API error \((?:408|429|500|502|503|504)\)/.test(error.message)) {
        return true;
    }
    if (error instanceof TypeError && /fetch failed/i.test(error.message)) {
        return true;
    }

    const cause = error.cause;
    return cause instanceof Error && /\b(?:ECONNRESET|ECONNREFUSED|ETIMEDOUT|ENOTFOUND|EAI_AGAIN)\b/i.test(cause.message);
}

export class ScrappaClient {
    private readonly apiKey: string;
    private readonly baseUrl: string;
    private readonly debug: boolean;
    private readonly timeoutMs: number;
    private readonly retryDelayMs?: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
        this.timeoutMs = config.timeoutMs ?? 45000;
        this.retryDelayMs = config.retryDelayMs;
    }

    async get<T>(endpoint: string, params: Record<string, string>, options: { attempts?: number } = {}): Promise<T> {
        const attempts = Math.max(1, options.attempts ?? 1);
        let lastError: unknown;

        for (let attempt = 1; attempt <= attempts; attempt += 1) {
            try {
                return await this.send<T>(endpoint, params);
            } catch (error) {
                lastError = error;
                if (attempt >= attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const delayMs = this.retryDelayMs ?? getRetryDelayMs(attempt);
                console.warn(`Scrappa directions request failed. Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(endpoint: string, params: Record<string, string>): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);
        for (const [key, value] of Object.entries(params)) {
            if (value !== undefined && value !== null && value !== '') {
                url.searchParams.set(key, value);
            }
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);
        try {
            if (this.debug) {
                console.log(`[Scrappa] GET ${url.toString()}`);
            }

            const response = await fetch(url.toString(), {
                method: 'GET',
                headers: {
                    'X-API-Key': this.apiKey,
                    Accept: 'application/json',
                    'User-Agent': 'thescrappa-google-maps-directions-scraper/1.0',
                },
                signal: controller.signal,
            });
            if (!response.ok) {
                throw new Error(`Scrappa API error (${response.status}): ${await this.readErrorMessage(response)}`);
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
        try {
            const body = await response.text();
            if (!body) {
                return fallback;
            }
            try {
                const parsed = JSON.parse(body) as { message?: string; errors?: Record<string, string[]> };
                const details = parsed.errors
                    ? Object.entries(parsed.errors).map(([field, messages]) => `${field}: ${messages.join(', ')}`).join('; ')
                    : '';
                return [parsed.message ?? fallback, details].filter(Boolean).join(' - ');
            } catch {
                return body.replace(/\s+/g, ' ').trim().slice(0, 500);
            }
        } catch {
            return fallback;
        }
    }
}
