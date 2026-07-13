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
    errors?: Record<string, string[]>;
}

export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) {
        super(`Scrappa API request timed out after ${timeoutMs}ms`, options);
        this.name = 'ScrappaTimeoutError';
    }
}

export class ScrappaAuthError extends Error {
    readonly status: 401 | 403;

    constructor(status: 401 | 403, message: string, options?: ErrorOptions) {
        super(`Scrappa API error (${status}): ${message}`, options);
        this.name = 'ScrappaAuthError';
        this.status = status;
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

    return error.cause instanceof Error && /\b(?:ECONNRESET|ECONNREFUSED|ETIMEDOUT|ENOTFOUND|EAI_AGAIN)\b/i.test(error.cause.message);
}

export class ScrappaClient {
    private readonly apiKey: string;
    private readonly baseUrl: string;
    private readonly debug: boolean;
    private readonly timeoutMs: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
        this.timeoutMs = config.timeoutMs ?? 60000;
    }

    async get<T>(endpoint: string, params: Record<string, unknown> = {}, options: ScrappaRequestOptions = {}): Promise<T> {
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

                const delayMs = getRetryDelayMs(attempt - 1);
                console.warn(`Scrappa API request failed (${this.describeError(error)}). Retrying attempt ${attempt + 1}/${attempts} in ${delayMs}ms.`);
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }

        throw lastError;
    }

    private async send<T>(endpoint: string, params: Record<string, unknown>): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);
        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== null && value !== '') {
                url.searchParams.set(key, typeof value === 'boolean' ? (value ? '1' : '0') : String(value));
            }
        });

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            const response = await fetch(url.toString(), {
                method: 'GET',
                headers: {
                    'X-API-Key': this.apiKey,
                    'Accept': 'application/json',
                    'User-Agent': 'thescrappa-vinted-user-profile-scraper/1.0',
                },
                signal: controller.signal,
            });

            if (!response.ok) {
                const message = await this.readErrorMessage(response);
                if (response.status === 401 || response.status === 403) {
                    throw new ScrappaAuthError(response.status, message);
                }
                throw new Error(`Scrappa API error (${response.status}): ${message}`);
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
        return error instanceof Error ? error.message : String(error);
    }

    private async readErrorMessage(response: Response): Promise<string> {
        const fallback = response.statusText || `HTTP ${response.status}`;
        let bodyText: string;
        try {
            bodyText = await response.text();
        } catch {
            return fallback;
        }

        if (!bodyText) {
            return fallback;
        }

        try {
            const errorData = JSON.parse(bodyText) as ScrappaError;
            let message = errorData.message ?? fallback;
            if (errorData.errors) {
                const details = Object.entries(errorData.errors).map(([field, messages]) => `${field}: ${messages.join(', ')}`).join('; ');
                if (details) {
                    message += ` - ${details}`;
                }
            }
            return message;
        } catch {
            return bodyText.replace(/\s+/g, ' ').trim().slice(0, 500);
        }
    }
}
