import { Actor } from 'apify';
import axios from 'axios';

/**
 * Searches for playlists based on a query string.
 * This is the main function for this Actor, as defined by the input schema.
 * @param {string} query The search query string.
 * @param {string} sort Sort order (optional)
 * @param {number} limit Number of results to return (optional)
 * @param {string} continuation Continuation token for pagination (optional)
 */
async function searchPlaylists(query, sort = 'relevance', limit = 20, continuation = '') {
    // Validate that the required query parameter is present.
    if (!query) {
        throw new Error('Search query "q" not provided. Please provide a value for "searchPlaylistQuery" in the input.');
    }
    
    console.log(`Running action: Search Playlists for query: "${query}"`);
    
    // Construct the base API URL with required parameters
    let apiUrl = `https://ytapi.scrappa.co/search/playlists?q=${encodeURIComponent(query)}`;
    
    // Add optional parameters only if they have valid values
    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        apiUrl += `&sort=${encodeURIComponent(sort)}`;
    }

    if (limit && typeof limit === 'number' && limit > 0) {
        apiUrl += `&limit=${encodeURIComponent(limit)}`;
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const data = response.data['results'];
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} playlists for query: ${query}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch playlists for query: ${query}`, error.message);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    // Get the input, expecting only the 'searchPlaylistQuery' field.
    const input = (await Actor.getInput()) || {};
    const { q, sort, limit, continuation } = input;

    // Directly call the function with the input, as there is only one possible task.
    await searchPlaylists(q, sort, limit, continuation);

    // Gracefully exit the Actor process.
    await Actor.exit();
});