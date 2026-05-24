import { Actor } from 'apify';
import axios from 'axios';

async function searchHashtag(query, sort = 'relevance', limit, duration, upload_date, continuation = '', contentType, features) {
    // Validate that the required query parameter is present.
    if (!query) {
        throw new Error('Search query "hashtag" not provided. Please provide a value for "searchHashtag" in the input.');
    }
    
    // Construct the base API URL with required parameters
    let apiUrl = `https://ytapi.scrappa.co/search/hashtag?hashtag=${encodeURIComponent(query)}`;
    
    // Add optional parameters only if they have valid values
    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        apiUrl += `&sort=${encodeURIComponent(sort)}`;
    }

    if (limit && typeof limit === 'number' && limit > 0) {
        apiUrl += `&limit=${encodeURIComponent(limit)}`;
    }

     if (duration && typeof duration === 'string' && duration.trim() !== '') {
        apiUrl += `&duration=${encodeURIComponent(duration)}`;
    }


    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

     if (upload_date && typeof upload_date === 'string' && upload_date.trim() !== '') {
        apiUrl += `&upload_date=${encodeURIComponent(upload_date)}`;
    }

    if (contentType && typeof contentType === 'string' && contentType.trim() !== '') {
        apiUrl += `&contentType=${encodeURIComponent(contentType)}`;
    }

    if (features && typeof features === 'string' && features.trim() !== '') {
        apiUrl += `&features=${encodeURIComponent(features)}`;
    }

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const data = response.data['results'];
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} hashtag for query: ${query}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch hashtag for query: ${query}`, error.message);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = (await Actor.getInput()) || {};
    const { hashtag, sort, duration, upload_date, limit, continuation, contentType, features } = input;

    // Directly call the function with the input, as there is only one possible task.
    await searchHashtag(hashtag, sort, limit, duration, upload_date, continuation, contentType, features);

    // Gracefully exit the Actor process.
    await Actor.exit();
});
