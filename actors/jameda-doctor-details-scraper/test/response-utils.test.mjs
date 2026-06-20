import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildJamedaDoctorDetailsDatasetItem,
    buildJamedaDoctorDetailsOutputSummary,
    unwrapJamedaDoctorDetailsResponse,
} = await import(responseUtilsModule);

const doctorUrl = 'https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin';

test('builds normalized Jameda doctor details dataset item', () => {
    const item = buildJamedaDoctorDetailsDatasetItem(
        {
            success: true,
            data: {
                basic_info: {
                    name: 'Markus Lietzau M.Sc.',
                    title: 'M.Sc.',
                    specialty: 'Zahnarzt',
                    profile_url: doctorUrl,
                    image_url: '//images.example/doctor.jpg',
                },
                description: 'Zahnarzt in Berlin',
                rating: {
                    rating: '1,0',
                    count: '52 Bewertungen',
                },
                clinic: {
                    name: 'Praxis Markus Lietzau M.Sc. Zahnarzt',
                },
                contact: {
                    phone: '+49 30 123456',
                    website: 'example.com',
                },
                address: {
                    street: 'Teststr. 1',
                    postal_code: '10115',
                    city: 'Berlin',
                },
                coordinates: {
                    latitude: '52,5200',
                    longitude: '13.4050',
                },
                opening_hours: { monday: '09:00-17:00' },
                services: ['Implantologie', 'Prophylaxe'],
                accepted_patients: ['Privat', 'Selbstzahler'],
                focus_areas: ['Zahnerhaltung'],
                conditions: ['Karies'],
                languages: ['Deutsch', 'Englisch'],
                booking_ids: { doctor_id: 'abc123' },
            },
            meta: {
                source: 'scrappa',
                scraped_at: '2026-06-20T00:00:00Z',
            },
        },
        {
            doctorUrl,
            params: {
                doctor_url: doctorUrl,
            },
        },
    );

    assert.equal(item.requested_doctor_url, doctorUrl);
    assert.equal(item.doctor_url, doctorUrl);
    assert.equal(item.doctor_name, 'Markus Lietzau M.Sc.');
    assert.equal(item.title, 'M.Sc.');
    assert.equal(item.specialty, 'Zahnarzt');
    assert.equal(item.description, 'Zahnarzt in Berlin');
    assert.equal(item.rating, '1,0');
    assert.equal(item.rating_number, 1);
    assert.equal(item.review_count, '52 Bewertungen');
    assert.equal(item.review_count_number, 52);
    assert.equal(item.clinic_name, 'Praxis Markus Lietzau M.Sc. Zahnarzt');
    assert.equal(item.phone, '+49 30 123456');
    assert.equal(item.website_url, 'https://example.com');
    assert.equal(item.address, 'Teststr. 1, 10115, Berlin');
    assert.equal(item.city, 'Berlin');
    assert.equal(item.postal_code, '10115');
    assert.equal(item.latitude, 52.52);
    assert.equal(item.longitude, 13.405);
    assert.equal(item.image_url, 'https://images.example/doctor.jpg');
    assert.equal(item.services_count, 2);
    assert.equal(item.focus_areas_count, 1);
    assert.equal(item.conditions_count, 1);
    assert.equal(item.languages_count, 2);
    assert.deepEqual(item.opening_hours, { monday: '09:00-17:00' });
    assert.deepEqual(item.booking_ids, { doctor_id: 'abc123' });
    assert.equal(item.request_doctor_url, doctorUrl);
    assert.equal(item.response_source, 'scrappa');
    assert.equal(item.scraped_at, '2026-06-20T00:00:00Z');
});

test('tolerates sparse doctor details responses', () => {
    const item = buildJamedaDoctorDetailsDatasetItem(
        {
            basic_info: {
                name: 'Example Doctor',
            },
            address: 'Berlin',
            rating: {
                score: 1.7,
                review_count: 4,
            },
        },
        {
            doctorUrl,
            params: {
                doctor_url: doctorUrl,
            },
        },
    );

    assert.equal(item.doctor_name, 'Example Doctor');
    assert.equal(item.doctor_url, doctorUrl);
    assert.equal(item.rating_number, 1.7);
    assert.equal(item.review_count_number, 4);
    assert.equal(item.address, 'Berlin');
    assert.equal(item.services_count, null);
    assert.equal(item.languages, null);
});

test('normalizes DE thousands separators without breaking decimal values', () => {
    const item = buildJamedaDoctorDetailsDatasetItem(
        {
            rating: {
                rating: '4,5',
                review_count: '1.234 Bewertungen',
            },
            coordinates: {
                latitude: '52,5200',
                longitude: '13.4050',
            },
        },
        {
            doctorUrl,
            params: {
                doctor_url: doctorUrl,
            },
        },
    );

    assert.equal(item.rating_number, 4.5);
    assert.equal(item.review_count_number, 1234);
    assert.equal(item.latitude, 52.52);
    assert.equal(item.longitude, 13.405);
});

test('normalizes count separators without treating decimal coordinates as thousands', () => {
    const item = buildJamedaDoctorDetailsDatasetItem(
        {
            rating: {
                review_count: '1,234 reviews',
            },
            coordinates: {
                latitude: '52.520',
                longitude: '13.405',
            },
        },
        {
            doctorUrl,
            params: {
                doctor_url: doctorUrl,
            },
        },
    );

    assert.equal(item.review_count_number, 1234);
    assert.equal(item.latitude, 52.52);
    assert.equal(item.longitude, 13.405);
});

test('unwraps live Scrappa response shape', () => {
    const response = {
        success: true,
        message: 'Doctor details retrieved successfully',
        data: {
            basic_info: {
                name: 'Markus Lietzau M.Sc.',
            },
        },
    };

    assert.deepEqual(unwrapJamedaDoctorDetailsResponse(response), response.data);
});

test('builds OUTPUT summary without uncharged response payloads', () => {
    const summary = buildJamedaDoctorDetailsOutputSummary({
        doctorUrls: [doctorUrl, 'https://www.jameda.de/example/aerztin/hamburg'],
        savedProfiles: 1,
        failures: [{ doctor_url: 'https://www.jameda.de/example/aerztin/hamburg', error: '404' }],
        statusMessage: '1 of 2 Jameda doctor detail request(s) failed.',
    });

    assert.deepEqual(summary, {
        request: {
            endpoint: '/jameda/doctor-details',
            doctor_urls: [doctorUrl, 'https://www.jameda.de/example/aerztin/hamburg'],
        },
        doctors_requested: 2,
        doctors_saved: 1,
        doctors_failed: 1,
        responses_saved: 1,
        status_message: '1 of 2 Jameda doctor detail request(s) failed.',
        failures: [{ doctor_url: 'https://www.jameda.de/example/aerztin/hamburg', error: '404' }],
    });
});
