import { buildGoogleTrendsAutocompleteParams } from './request-params.js';
import type { ScrappaClient } from './shared/index.js';

export interface AutocompleteSummary {
    response: Record<string, unknown> | null;
    error: string | null;
}

export async function fetchAutocompleteSummary(
    client: Pick<ScrappaClient, 'get'>,
    params: Record<string, unknown>,
    attempts: number,
): Promise<AutocompleteSummary> {
    try {
        const response = await client.get<Record<string, unknown>>(
            '/google-trends/autocomplete',
            buildGoogleTrendsAutocompleteParams(params),
            { attempts },
        );

        return { response, error: null };
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.warn(`Google Trends autocomplete summary failed: ${message}`);
        return { response: null, error: message };
    }
}
