import { Actor } from 'apify';
import axios from 'axios';

const SCRAPPA_API_URL = 'https://scrappa.co/api/instagram/user';

function normalizeUsername(username) {
    return username.trim().replace(/^@+/, '');
}

function flattenProfile(response) {
    const user = response?.user ?? response?.data?.user ?? response?.data ?? response;

    if (!user || typeof user !== 'object') {
        return response;
    }

    return {
        ...response,
        ...user,
    };
}

Actor.main(async () => {
    const input = await Actor.getInput();
    const username = typeof input?.username === 'string' ? normalizeUsername(input.username) : '';

    if (!username) {
        await Actor.fail('Instagram username is required.');
        return;
    }

    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        await Actor.fail('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        return;
    }

    console.log(`Fetching Instagram user info for: ${username}`);

    try {
        const response = await axios.get(SCRAPPA_API_URL, {
            params: { username },
            headers: {
                'X-API-KEY': apiKey,
                Accept: 'application/json',
            },
            timeout: 90000,
            validateStatus: (status) => status < 500,
        });

        if (response.status === 401 && response.data?.code === 'UNAUTHORIZED') {
            await Actor.fail('Scrappa API authentication failed. Check the SCRAPPA_API_KEY Actor secret.');
            return;
        }

        const item = flattenProfile(response.data);
        await Actor.pushData(item);
        if (response.status >= 400 || response.data?.success === false) {
            console.warn(`Scrappa returned a structured error for ${username}: ${response.data?.error ?? response.status}`);
        } else {
            console.log(`Successfully fetched Instagram user info for: ${username}`);
        }
    } catch (error) {
        const status = error.response?.status;
        const message = error.response?.data?.message
            ?? error.response?.data?.error
            ?? error.message
            ?? String(error);
        const statusPrefix = status ? `Scrappa API returned HTTP ${status}: ` : '';

        await Actor.fail(`${statusPrefix}${message}`);
    }
});
