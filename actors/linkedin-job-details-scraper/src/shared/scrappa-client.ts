export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    timeoutMs?: number;
}

export interface ScrappaError {
    status: number;
    message: string;
    errors?: Record<string, string[]>;
}

export class ScrappaApiError extends Error {
    constructor(
        public readonly status: number,
        message: string,
        public readonly errors?: Record<string, string[]>,
    ) {
        super(`Scrappa API error (${status}): ${message}`);
        this.name = 'ScrappaApiError';
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

    async get<T>(endpoint: string, params: Record<string, unknown> = {}): Promise<T> {
        return this.request<T>('GET', endpoint, params);
    }

    private async request<T>(
        method: 'GET',
        endpoint: string,
        params: Record<string, unknown> = {},
    ): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);

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

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
        };

        if (this.debug) {
            console.log(`[Scrappa] ${method} ${url.toString()}`);
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        let response: Response;
        try {
            response = await fetch(url.toString(), {
                method,
                headers,
                signal: controller.signal,
            });
        } catch (error) {
            clearTimeout(timeoutId);
            if (error instanceof Error && error.name === 'AbortError') {
                throw new Error(`Scrappa API request timed out after ${this.timeoutMs}ms`);
            }
            throw error;
        }
        clearTimeout(timeoutId);

        if (!response.ok) {
            const responseClone = response.clone();
            let errorMessage: string;
            let errors: Record<string, string[]> | undefined;

            try {
                const errorData = await response.json() as { message?: string; errors?: Record<string, string[]> };
                errorMessage = errorData.message ?? `HTTP ${response.status}`;
                errors = errorData.errors;

                if (errors) {
                    const errorDetails = Object.entries(errors)
                        .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                        .join('; ');
                    errorMessage += ` - ${errorDetails}`;
                }
            } catch {
                errorMessage = await responseClone.text() || `HTTP ${response.status}`;
            }

            throw new ScrappaApiError(response.status, errorMessage, errors);
        }

        return response.json() as Promise<T>;
    }
}
