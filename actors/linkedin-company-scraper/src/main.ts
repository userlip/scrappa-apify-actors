import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { buildLinkedInCompanyParams } from './request-params.js';
import { normalizeLinkedInCompanyUrl } from './url.js';

interface LinkedInCompanyInput {
    url: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Employee {
    name: string;
    title: string;
    profile_url: string;
}

interface Post {
    text: string;
    date: string;
    likes: number;
    comments: number;
}

interface Location {
    name: string;
    type: string;
}

interface SimilarPage {
    name: string;
    url: string;
}

interface Funding {
    round: string;
    amount: string;
    date: string;
}

interface Address {
    street?: string;
    city?: string;
    state?: string;
    country?: string;
    postal_code?: string;
}

interface LinkedInCompanyResponse {
    success: boolean;
    name?: string;
    description?: string;
    logo?: string;
    website?: string;
    employee_count?: number;
    address?: Address[];
    posts?: Post[];
    followers?: number;
    similar_pages?: SimilarPage[];
    specialties?: string[];
    employees?: Employee[];
    funding?: Funding | null;
    industry?: string;
    size?: string;
    type?: string;
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
}

type LinkedInCompanyResult = LinkedInCompanyResponse & { url?: string };

async function persistActorResult(result: LinkedInCompanyResult): Promise<void> {
    await Actor.pushData(result);
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', result);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInCompanyInput>();
        if (!input?.url) {
            throw new Error('LinkedIn company URL is required');
        }

        const normalizedUrl = normalizeLinkedInCompanyUrl(input.url);
        console.log('Scraping LinkedIn company: "' + normalizedUrl + '"');
        if (normalizedUrl !== input.url) {
            console.log('(normalized from: ' + input.url + ')');
        }

        const client = new ScrappaClient({ apiKey });
        const params = buildLinkedInCompanyParams({
            url: normalizedUrl,
            use_cache: input.use_cache,
            maximum_cache_age: input.maximum_cache_age,
        });

        let response: LinkedInCompanyResponse;

        try {
            response = await client.get<LinkedInCompanyResponse>('/linkedin/company', params);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);

            // Handle 404s gracefully - push a failure result instead of failing the actor
            if (message.includes('(404)')) {
                console.log('Company not found (404): ' + normalizedUrl);
                const failResult = { success: false, url: normalizedUrl, message: 'Company not found', status_code: 404 };
                await persistActorResult(failResult);
                await Actor.exit();
                return;
            }

            throw error;
        }

        if (!response.success) {
            // Keep the run green when Scrappa returns a structured failure payload.
            if (response.status_code === 404) {
                console.log('Company not found: ' + normalizedUrl);
            }

            console.warn('Company scraping returned success: false' + (response.message ? ` (${response.message})` : ''));
            const failedResult = {
                url: normalizedUrl,
                ...response,
            };
            await persistActorResult(failedResult);
            await Actor.exit();
            return;
        }

        // Push company data to dataset
        await persistActorResult(response);
        console.log('Successfully scraped company: ' + (response.name ?? 'Unknown'));

        // Log summary
        console.log('LinkedIn Company scrape completed successfully');

        const summary = {
            name: response.name ?? 'Unknown',
            industry: response.industry ?? 'Unknown',
            followers: response.followers ?? 0,
            employee_count: response.employee_count ?? 0,
            employees_found: response.employees?.length ?? 0,
            posts_found: response.posts?.length ?? 0,
            locations_found: response.address?.length ?? 0,
            specialties_count: response.specialties?.length ?? 0,
            similar_pages_count: response.similar_pages?.length ?? 0,
            has_funding: !!response.funding,
            cached: response.cached ?? false,
        };

        console.log('Results summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();
