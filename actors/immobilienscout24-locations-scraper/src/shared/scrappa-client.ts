export interface ScrappaConfig {
    apiKey: string;
    timeoutMs: number;
}

export class ScrappaApiError extends Error {
    constructor(public readonly status: number, public readonly responseMessage: string) {
        super(`Scrappa API error (${status}): ${responseMessage}`);
        this.name = 'ScrappaApiError';
    }
}

export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) {
        super(`Scrappa API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaTimeoutError';
    }
}

export function isRetryableScrappaError(error: unknown): boolean {
    return error instanceof ScrappaTimeoutError
        || (error instanceof ScrappaApiError && [408, 429, 500, 502, 503, 504].includes(error.status));
}

export class ScrappaClient {
    constructor(private readonly config: ScrappaConfig) {}

    async get<T>(endpoint: string, params: Record<string, unknown>, attempts = 1): Promise<T> {
        let lastError: unknown;

        for (let attempt = 1; attempt <= attempts; attempt += 1) {
            try {
                return await this.send<T>(endpoint, params);
            } catch (error) {
                lastError = error;
                if (attempt === attempts || !isRetryableScrappaError(error)) {
                    break;
                }

                const delayMs = getRetryDelayMs(attempt);
                console.warn(`Scrappa request failed (${describeError(error)}). Retrying ${attempt + 1}/${attempts} in ${Math.round(delayMs)}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(endpoint: string, params: Record<string, unknown>): Promise<T> {
        const url = new URL(`https://scrappa.co/api${endpoint}`);
        for (const [key, value] of Object.entries(params)) {
            url.searchParams.set(key, String(value));
        }

        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), this.config.timeoutMs);

        try {
            const response = await fetch(url, {
                headers: {
                    'X-API-Key': this.config.apiKey,
                    Accept: 'application/json',
                    'User-Agent': 'thescrappa-immobilienscout24-locations-scraper/1.0',
                },
                signal: controller.signal,
            });
            if (!response.ok) {
                throw new ScrappaApiError(response.status, await readErrorMessage(response));
            }
            return await response.json() as T;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new ScrappaTimeoutError(this.config.timeoutMs, { cause: error });
            }
            throw error;
        } finally {
            clearTimeout(timeout);
        }
    }
}

export function getRetryDelayMs(failedAttempt: number, jitterMs = Math.random() * 1000): number {
    return Math.min(1000 * 2 ** failedAttempt + jitterMs, 10000);
}

export function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

async function readErrorMessage(response: Response): Promise<string> {
    const text = (await response.text()).replace(/\s+/g, ' ').trim();
    if (!text) {
        return response.statusText || `HTTP ${response.status}`;
    }

    try {
        const body = JSON.parse(text) as { message?: string };
        return body.message ?? text.slice(0, 500);
    } catch {
        return text.slice(0, 500);
    }
}
