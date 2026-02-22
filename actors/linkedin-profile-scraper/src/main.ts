import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface LinkedInProfileInput {
    url: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Experience {
    company?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

interface Education {
    school?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

interface Article {
    title?: string;
    url?: string;
    published_date?: string;
    [key: string]: unknown;
}

interface Activity {
    type?: string;
    text?: string;
    date?: string;
    [key: string]: unknown;
}

interface Publication {
    title?: string;
    publisher?: string;
    date?: string;
    [key: string]: unknown;
}

interface Project {
    title?: string;
    description?: string;
    date?: string;
    [key: string]: unknown;
}

interface Recommendation {
    name?: string;
    title?: string;
    text?: string;
    [key: string]: unknown;
}

interface SimilarProfile {
    name?: string;
    url?: string;
    title?: string;
    [key: string]: unknown;
}

interface LinkedInProfileResponse {
    success: boolean;
    name?: string;
    image?: string;
    location?: string;
    followers?: number;
    connections?: number;
    about?: string;
    job_titles?: string[];
    experience?: Experience[];
    education?: Education[];
    skills?: string[];
    articles?: Article[];
    activity?: Activity[];
    publications?: Publication[];
    projects?: Project[];
    recommendations?: Recommendation[];
    similar_profiles?: SimilarProfile[];
    cached?: boolean;
    cached_at?: string;
    [key: string]: unknown;
}

/**
 * Normalize a LinkedIn profile URL:
 * - Replace country subdomains (de., uk., fr., etc.) with www.
 * - Strip query parameters
 * - Ensure trailing slash consistency
 */
function normalizeLinkedInUrl(rawUrl: string): string {
    const parsed = new URL(rawUrl);

    // Replace country subdomains with www
    parsed.hostname = parsed.hostname.replace(/^[a-z]{2,3}\.linkedin\.com$/, 'www.linkedin.com');

    // Strip query parameters
    parsed.search = '';

    // Strip hash
    parsed.hash = '';

    // Ensure the path does not have a trailing slash for consistency
    parsed.pathname = parsed.pathname.replace(/\/+$/, '');

    return parsed.toString();
}

async function main(): Promise<void> {
    await Actor.init();

    let shouldExit = true;

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInProfileInput>();
        if (!input?.url) {
            throw new Error('LinkedIn profile URL is required');
        }

        // Normalize the URL before sending to the API
        const normalizedUrl = normalizeLinkedInUrl(input.url);
        console.log(`Fetching LinkedIn profile: ${normalizedUrl}`);
        if (normalizedUrl !== input.url) {
            console.log(`(normalized from: ${input.url})`);
        }

        const client = new ScrappaClient({ apiKey });

        const params: Record<string, unknown> = {
            url: normalizedUrl,
            maximum_cache_age: input.maximum_cache_age,
        };

        if (input.use_cache) {
            params.use_cache = 1;
        }

        const response = await client.get<LinkedInProfileResponse>('/linkedin/profile', params);

        // Push the entire profile as a single dataset item
        if (response.success) {
            await Actor.pushData(response);
            console.log(`Successfully scraped profile: ${response.name || 'Unknown'}`);
        } else {
            console.warn('Profile scraping returned success: false');
            await Actor.pushData(response);
        }

        // Store full response in key-value store for complete data access
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        // Log summary
        console.log('LinkedIn profile scraping completed');

        const summary = {
            success: response.success,
            name: response.name,
            location: response.location,
            followers: response.followers ?? 0,
            connections: response.connections ?? 0,
            experience_count: response.experience?.length ?? 0,
            education_count: response.education?.length ?? 0,
            skills_count: response.skills?.length ?? 0,
            articles_count: response.articles?.length ?? 0,
            activity_count: response.activity?.length ?? 0,
            publications_count: response.publications?.length ?? 0,
            projects_count: response.projects?.length ?? 0,
            recommendations_count: response.recommendations?.length ?? 0,
            similar_profiles_count: response.similar_profiles?.length ?? 0,
            is_cached: response.cached ?? false,
        };

        console.log('Profile summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);

        // Handle 404s gracefully - push a failed result instead of failing the actor
        if (message.includes('(404)')) {
            console.warn(`Profile not found (404): ${message}`);
            const failedResult = {
                success: false,
                error: 'Profile not found',
                message,
            };
            await Actor.pushData(failedResult);
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', failedResult);
        } else {
            console.error('Actor failed: ' + message);
            await Actor.fail(message);
            shouldExit = false;
            return;
        }
    }

    if (shouldExit) {
        await Actor.exit();
    }
}

main();
