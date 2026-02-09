import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

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

    // Validate URL format
    const urlPattern = /^https?:\/\/(www\.)?linkedin\.com\/company\/[a-zA-Z0-9_-]+\/?$/i;
    if (!urlPattern.test(input.url)) {
        throw new Error('Invalid LinkedIn company URL. Expected format: https://www.linkedin.com/company/company-slug');
    }

    console.log('Scraping LinkedIn company: "' + input.url + '"');

    const client = new ScrappaClient({ apiKey });

    const params: Record<string, unknown> = {
        url: input.url,
    };

    if (input.use_cache) {
        params.use_cache = 1;
    }

    if (input.maximum_cache_age !== undefined) {
        params.maximum_cache_age = input.maximum_cache_age;
    }

    const response = await client.get<LinkedInCompanyResponse>('/linkedin/company', params);

    if (!response.success) {
        throw new Error(response.message ?? 'Failed to scrape LinkedIn company');
    }

    // Push company data to dataset
    await Actor.pushData(response);
    console.log('Successfully scraped company: ' + (response.name ?? 'Unknown'));

    // Store full response in key-value store for complete data access
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

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
}

await Actor.exit();
