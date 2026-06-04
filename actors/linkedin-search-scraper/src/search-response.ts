export interface LinkedInSearchResult {
    position?: number;
    title?: string;
    link?: string;
    displayed_link?: string;
    snippet?: string | null;
    snippet_highlighted_words?: string[];
    thumbnail?: string | null;
    rich_snippet?: Record<string, unknown> | null;
    [key: string]: unknown;
}

export interface LinkedInSearchResponse {
    organic_results?: LinkedInSearchResult[];
    people_also_search_for?: unknown[];
    search_information?: {
        query_displayed?: string;
        total_results?: number;
        time_taken?: number;
        [key: string]: unknown;
    };
    pagination?: {
        current_page?: number;
        pages?: unknown[];
        [key: string]: unknown;
    };
    total_results?: number;
    [key: string]: unknown;
}

export function getLinkedInSearchResults(response: LinkedInSearchResponse): LinkedInSearchResult[] {
    return Array.isArray(response.organic_results) ? response.organic_results : [];
}
