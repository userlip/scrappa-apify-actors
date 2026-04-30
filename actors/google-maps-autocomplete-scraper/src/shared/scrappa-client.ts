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
                    // Convert booleans to 1/0 for Laravel API compatibility
                    if (typeof value === 'boolean') {
                        url.searchParams.set(key, value ? '1' : '0');
                    } else {
                        url.searchParams.set(key, String(value));
                    }
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
            const response = await fetch(url.toString(), {
                ...options,
                signal: controller.signal,
            });

            if (!response.ok) {
                let errorMessage: string;
                const responseClone = response.clone();
                try {
                    const errorData = await response.json() as { message?: string; errors?: Record<string, string[]> };
                    errorMessage = errorData.message ?? `HTTP ${response.status}`;

                    if (errorData.errors) {
                        const errorDetails = Object.entries(errorData.errors)
                            .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                            .join('; ');
                        errorMessage += ` - ${errorDetails}`;
                    }
                } catch {
                    errorMessage = await responseClone.text() || `HTTP ${response.status}`;
                }

                throw new Error(`Scrappa API error (${response.status}): ${errorMessage}`);
            }

            return response.json() as Promise<T>;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new Error(`Scrappa API request timed out after ${this.timeoutMs}ms`);
            }
            throw error;
        } finally {
            clearTimeout(timeoutId);
        }
    }
}
