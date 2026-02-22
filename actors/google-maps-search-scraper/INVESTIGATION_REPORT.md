# Google Maps Search API - Scrappa vs Apify Actor Investigation

## EXECUTIVE SUMMARY

The Apify actor for Google Maps Search is **significantly incomplete** compared to the Scrappa API. The actor is using a **simplified interface** that only captures a fraction of the available data fields and parameters. Multiple important parameters and response fields are missing.

---

## 1. PARAMETER COMPARISON

### Scrappa API Parameters (SimpleSearchRequest)

**Required Parameters:**
- `query` (string) - The search term that the API will use

**Optional Parameters:**
- `hl` (string, nullable) - Language code for results
  - Format: ISO 639-1 two-letter language code or with region (e.g., en, de, en-US)
  - Default: 'en'
  - Validation: regex `/^[a-zA-Z]{2}(-[a-zA-Z]{2})?$/`
  - Examples: en, de, fr, es, en-US, de-DE

- `gl` (string, nullable) - Country/region code for geo-filtering
  - Format: ISO 3166-1 alpha-2 country code (2-letter exactly)
  - Validation: regex `/^[a-zA-Z]{2}$/`
  - Examples: us, de, uk, jp

- `debug` (optional) - Debug mode flag (boolean equivalent)
  - Only enabled for Super Admin or Admin users

### Apify Actor Input Parameters (input_schema.json)

**Required Parameters:**
- `query` (string) - Search query ✓ MATCHES

**Optional Parameters:**
- `language` (string, optional, default: "en")
  - Description: Language code (e.g., 'en', 'de', 'es')
  - maxLength: 5
  - **ISSUE**: Parameter name is `language` but API uses `hl`
  - **ISSUE**: Validation is missing (accepts any 5-char string, no ISO format enforcement)

- `country` (string, optional)
  - Description: Two-letter country code
  - maxLength: 2
  - **ISSUE**: Parameter name is `country` but API uses `gl`
  - **ISSUE**: Validation is missing

**MISSING in Apify Actor:**
- `debug` parameter is completely missing

### Parameter Mapping Issues in main.ts

```typescript
// Current mapping (line 43-47)
const response = await client.get<GoogleMapsSearchResponse>('/maps/simple-search', {
    query: input.query,
    hl: input.language || 'en',    // ✓ Correctly maps language → hl
    gl: input.country,              // ✓ Correctly maps country → gl
});
```

**Status**: Parameter mapping in main.ts is CORRECT, but input schema uses wrong names.

---

## 2. RESPONSE STRUCTURE COMPARISON

### Scrappa API Response (from GoogleMapsApiController docblock)

The `simpleSearch` method returns:

```php
array{
  items: array<int, array{
    name: string|null,
    price_level: string|null,
    price_level_text: string|null,
    review_count: int,
    rating: float|null,
    website: string|null,
    domain: string|null,
    latitude: float|null,
    longitude: float|null,
    business_id: string|null,
    subtypes: array<string>|null,
    district: string|null,
    full_address: string|null,
    timezone: string|null,
    short_description: string|null,
    full_description: string|null,
    owner_id: string|null,
    owner_name: string|null,
    owner_link: string|null,
    order_link: string|null,
    google_mid: string|null,
    type: string|null,
    phone_numbers: array<string>,
    place_id: string|null,
    photos_sample: array<int, array{
      photo_id: string|null,
      photo_url: string|null,
      photo_url_large: string|null,
      video_thumbnail_url: string|null,
      latitude: float|null,
      longitude: float|null,
      type: string
    }>,
    opening_hours: array<int, array{
      day: string,
      hours: array<string>,
      date: string|null,
      special_day: string|null
    }>|null,
    current_status: string|null
  }>
}
```

### Apify Actor Response (from main.ts and actor.json)

**In main.ts (lines 49-55):**
```typescript
if (response.results && response.results.length > 0) {
    await Actor.pushData(response.results);
    console.log(`Found ${response.results.length} results`);
}

const store = await Actor.openKeyValueStore();
await store.setValue('OUTPUT', response);
```

**Issues:**
1. Expects response.results array - but Scrappa API returns `items` array (not `results`)
2. Uses generic BusinessResult interface which only captures:
   - name
   - rating
   - review_count
   - address
   - phone_numbers
   - website
   - type
   - business_id

**In actor.json (transformation fields):**
```json
"transformation": {
  "fields": ["name", "rating", "review_count", "address", "phone", "website", "type"]
}
```

**Issues:**
1. Field mapping is incomplete
2. Displays only 7 fields from 30+ available fields
3. Uses `phone` instead of `phone_numbers`

### MISSING Response Fields

The Apify actor completely ignores these Scrappa API response fields:

**Location & Geography:**
- price_level
- price_level_text
- latitude / longitude
- district
- full_address
- timezone

**Business Details:**
- business_id
- subtypes
- short_description
- full_description
- owner_id
- owner_name
- owner_link
- order_link
- google_mid
- place_id
- domain (website domain extracted)
- current_status

**Complex Objects:**
- photos_sample (array with photo_id, photo_url, photo_url_large, video_thumbnail_url, latitude, longitude, type)
- opening_hours (array with day, hours, date, special_day)

---

## 3. CODE ISSUES IN main.ts

### Issue #1: Wrong Response Property Name
```typescript
// Line 49 - WRONG
if (response.results && response.results.length > 0) {
```

**Expected:** `response.items` (based on Scrappa API response)
**Current:** `response.results`
**Impact:** Data will not be pushed to Actor dataset; will silently fail

### Issue #2: Incomplete Data Type Definition
```typescript
interface BusinessResult {
    name: string;
    rating?: number;
    review_count?: number;
    address: string;
    phone_numbers?: string[];
    website?: string;
    type?: string;
    business_id?: string;
    [key: string]: unknown;  // Catch-all prevents proper typing
}
```

**Problems:**
- Missing 20+ fields documented in API
- `address` marked as required, but API returns it as `full_address` and nullable
- Using `[key: string]: unknown` defeats type safety
- No structure for complex objects (photos, hours)

### Issue #3: Shallow Data Transformation
```typescript
await Actor.pushData(response.results);
```

**Problem:** Pushes raw results without any transformation or flattening. This means:
- Complex nested objects (photos_sample, opening_hours) are included as-is
- No field mapping (e.g., full_address → address)
- Dataset transformation in actor.json references non-existent fields

---

## 4. INPUT SCHEMA ISSUES (input_schema.json)

### Issue #1: Wrong Parameter Names
```json
{
  "language": { ... },    // ✗ Should be "hl"
  "country": { ... }      // ✗ Should be "gl"
}
```

**Impact:** User-facing labels are confusing (input_schema says "language" but Scrappa expects "hl")

### Issue #2: Missing Validation
```json
{
  "language": {
    "type": "string",
    "maxLength": 5        // ✗ No regex pattern validation
  },
  "country": {
    "type": "string",
    "maxLength": 2        // ✗ Allows any 2-char string
  }
}
```

**Expected validation:**
- `language`: `/^[a-zA-Z]{2}(-[a-zA-Z]{2})?$/` (ISO 639-1 format)
- `country`: `/^[a-zA-Z]{2}$/` (ISO 3166-1 alpha-2 format)

### Issue #3: Missing Debug Parameter
```json
{
  "required": ["query"]
  // ✗ Missing debug parameter definition
}
```

**Note:** While debug mode requires Super Admin role, it should still be exposed as optional input parameter in schema.

---

## 5. CONFIGURATION ISSUES (actor.json)

### Issue #1: Dataset View Transformation Incomplete
```json
"transformation": {
  "fields": ["name", "rating", "review_count", "address", "phone", "website", "type"]
}
```

**Problems:**
1. `phone` should be `phone_numbers` (array of strings)
2. Missing high-value fields:
   - full_address, latitude, longitude (location data)
   - place_id, business_id (identifiers)
   - rating (already included ✓)
   - timezone (useful for scheduling)
   - opening_hours (business operations)
   - photos_sample (media)

### Issue #2: Memory Allocation
```json
"resources": {
  "memoryMbytes": 256
}
```

**Assessment:** 256MB is reasonable for simple search, but will be insufficient if processing large result sets with nested objects (photos, hours).

---

## 6. SUMMARY OF ISSUES

### Critical Issues
1. **Wrong response array property** - code looks for `response.results` but API returns `response.items`
2. **Parameter name mismatch** - input schema uses `language`/`country` but should use `hl`/`gl` for consistency
3. **Missing debug parameter** - not exposed in input schema

### High Priority Issues
1. Incomplete response data structure (missing 20+ fields)
2. No field mapping/transformation for complex objects
3. Missing validation rules for language and country parameters

### Medium Priority Issues
1. Dataset view only shows 7 fields from 30+ available
2. No documentation about full response structure
3. BusinessResult type definition incomplete

### Low Priority Issues
1. Memory allocation could be optimized
2. README documentation is generic, doesn't mention all features

---

## 7. COMPARISON TABLE

| Aspect | Scrappa API | Apify Actor | Status |
|--------|-------------|-------------|--------|
| **Required Parameters** | query | query | ✓ Match |
| **Optional Parameters** | hl, gl, debug | language, country | ✗ Partial match (wrong names) |
| **Response Root** | items array | results array | ✗ MISMATCH |
| **Response Fields** | 30+ fields documented | 8 fields mapped | ✗ Incomplete |
| **Photos Data** | photos_sample with 7 fields | Not captured | ✗ Missing |
| **Opening Hours** | Full structure with dates | Not captured | ✗ Missing |
| **Validation** | Regex patterns defined | None | ✗ Missing |
| **Parameter Case** | snake_case | camelCase → snake_case | ✓ Correct mapping in code |

---

## RECOMMENDATIONS

1. **Fix main.ts:** Change `response.results` to `response.items`
2. **Fix input_schema.json:** Rename parameters to `hl` and `gl` OR keep names but document the mapping
3. **Add validation:** Implement pattern validation for hl and gl parameters
4. **Add debug parameter** to input schema
5. **Expand BusinessResult interface:** Include all 30+ documented fields with proper typing
6. **Add field mapping:** Handle complex objects (photos, hours) appropriately
7. **Update actor.json:** Expand dataset view to include more valuable fields
8. **Update README:** Document all available response fields and their meanings

