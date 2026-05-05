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

                if (attempt >= attempts || !this.shouldRetry(error)) {
                    break;
                }

                const delayMs = 1000 * attempt;
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
                        if (value) {
                            url.searchParams.set(key, '1');
                        }
                    } else {
                        url.searchParams.set(key, String(value));
                    }
                }
            });
        }

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
            'User-Agent': 'thescrappa-google-jobs-scraper/1.0',
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

                throw new Error(`Scrappa API error (${response.status}): ${errorMessage}`);
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

    private shouldRetry(error: unknown): boolean {
        if (error instanceof ScrappaTimeoutError) {
            return true;
        }

        if (!(error instanceof Error)) {
            return false;
        }

        return /Scrappa API error \((?:408|429|500|502|503|504)\)/.test(error.message);
    }

    private describeError(error: unknown): string {
        if (error instanceof Error) {
            return error.message;
        }

        return String(error);
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
