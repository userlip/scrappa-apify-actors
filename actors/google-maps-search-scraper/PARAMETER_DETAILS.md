# Google Maps Search API - Detailed Parameter Analysis

## INPUT PARAMETERS

### Parameter 1: query

| Aspect | Scrappa API | Apify Actor |
|--------|------------|-------------|
| **Name** | query | query |
| **Type** | string | string |
| **Required** | YES | YES |
| **Default** | (none) | (none) |
| **Validation** | Required string | Required string |
| **Description** | The search term that the API will use | What to search for (e.g., 'restaurants in NYC', 'coffee shops', 'plumber') |
| **Prefill** | (none) | pizza restaurants |
| **Status** | ✓ MATCHES |

---

### Parameter 2: hl (Language)

| Aspect | Scrappa API | Apify Actor |
|--------|------------|-------------|
| **Parameter Name** | `hl` | `language` |
| **Type** | string | string |
| **Required** | NO | NO |
| **Default** | 'en' | 'en' |
| **Format** | ISO 639-1 two-letter or with region | Language code (e.g., 'en', 'de', 'es') |
| **Validation Pattern** | `^[a-zA-Z]{2}(-[a-zA-Z]{2})?$` | maxLength: 5 (NO PATTERN) |
| **Valid Examples** | en, de, fr, es, en-US, de-DE, pt-BR | en, de, es (implied) |
| **Mapping in Code** | (input param) | language → hl (correct mapping in main.ts) |
| **Status** | ✗ NAME MISMATCH, VALIDATION MISSING |

**Note:** The main.ts code correctly maps `language` to `hl` when calling the API, but the input schema uses the wrong name. This creates confusion for users.

---

### Parameter 3: gl (Country/Region)

| Aspect | Scrappa API | Apify Actor |
|--------|------------|-------------|
| **Parameter Name** | `gl` | `country` |
| **Type** | string | string |
| **Required** | NO | NO |
| **Default** | (uses coordinates from query) | (not specified) |
| **Format** | ISO 3166-1 alpha-2 (exactly 2 letters) | Two-letter country code |
| **Validation Pattern** | `^[a-zA-Z]{2}$` | maxLength: 2 (NO PATTERN) |
| **Valid Examples** | us, de, uk, jp, fr, es, au | us, uk, de (implied) |
| **Mapping in Code** | (input param) | country → gl (correct mapping in main.ts) |
| **Status** | ✗ NAME MISMATCH, VALIDATION MISSING |

**Note:** The main.ts code correctly maps `country` to `gl` when calling the API, but the input schema uses the wrong name.

---

### Parameter 4: debug

| Aspect | Scrappa API | Apify Actor |
|--------|------------|-------------|
| **Parameter Name** | `debug` | (MISSING) |
| **Type** | boolean/flag | N/A |
| **Required** | NO | N/A |
| **Authorization** | Super Admin or Admin only | N/A |
| **Function** | Enables debug logging in controller | N/A |
| **Status** | ✗ COMPLETELY MISSING |

**Note:** While the actor might not need debug mode, it should be exposed in the input schema as an optional parameter for consistency with the API.

---

## OUTPUT/RESPONSE FIELDS

### Scrappa API Returns (items array)

| Field | Type | Nullable | Description | In Actor |
|-------|------|----------|-------------|----------|
| name | string | Yes | Business name | ✓ |
| price_level | string | Yes | Price level indicator | ✗ |
| price_level_text | string | Yes | Text version of price level | ✗ |
| review_count | integer | No | Total number of reviews | ✓ |
| rating | float | Yes | Average rating (0-5) | ✓ |
| website | string | Yes | Business website URL | ✓ |
| domain | string | Yes | Domain extracted from website | ✗ |
| latitude | float | Yes | Latitude coordinate | ✗ |
| longitude | float | Yes | Longitude coordinate | ✗ |
| business_id | string | Yes | Google Business ID (0x...:0x...) | ✓ |
| subtypes | array<string> | Yes | Business category subtypes | ✗ |
| district | string | Yes | District name | ✗ |
| full_address | string | Yes | Complete address | ✗ CRITICAL |
| timezone | string | Yes | Business timezone | ✗ |
| short_description | string | Yes | Brief business description | ✗ |
| full_description | string | Yes | Complete business description | ✗ |
| owner_id | string | Yes | Business owner ID | ✗ |
| owner_name | string | Yes | Business owner name | ✗ |
| owner_link | string | Yes | Link to owner profile | ✗ |
| order_link | string | Yes | Link to order/booking | ✗ |
| google_mid | string | Yes | Google Machine ID | ✗ |
| type | string | Yes | Primary business type | ✓ |
| phone_numbers | array<string> | No | List of phone numbers | ✓ |
| place_id | string | Yes | Google Place ID | ✗ CRITICAL |
| photos_sample | array | Yes | Sample photos with metadata | ✗ CRITICAL |
| opening_hours | array | Yes | Operating hours by day | ✗ CRITICAL |
| current_status | string | Yes | Current open/closed status | ✗ |

**SUMMARY:**
- **Total Fields:** 28
- **Captured by Actor:** 8 (29% coverage)
- **Missing:** 20 fields (71% uncovered)
- **CRITICAL MISSING:** full_address, place_id, photos_sample, opening_hours

---

## PHOTOS_SAMPLE STRUCTURE (Nested)

The `photos_sample` array contains photo objects with:

```php
array<int, array{
    photo_id: string|null,           // Unique photo identifier
    photo_url: string|null,          // URL to standard size photo
    photo_url_large: string|null,    // URL to large size photo
    video_thumbnail_url: string|null,// Thumbnail if video
    latitude: float|null,            // Photo geolocation (if available)
    longitude: float|null,           // Photo geolocation (if available)
    type: string                     // Photo type (e.g., 'interior', 'exterior', 'menu')
}>
```

**Status:** Completely unhandled by Apify actor

---

## OPENING_HOURS STRUCTURE (Nested)

The `opening_hours` array contains:

```php
array<int, array{
    day: string,                  // Day of week (Monday, Tuesday, etc)
    hours: array<string>,         // Array of time ranges (e.g., ['9:00 - 17:00'])
    date: string|null,            // Specific date if special hours
    special_day: string|null      // Holiday or special occasion label
}>|null
```

**Status:** Completely unhandled by Apify actor

---

## CODE MAPPING VERIFICATION

### ScrappaClient call (main.ts lines 42-47)

```typescript
const response = await client.get<GoogleMapsSearchResponse>('/maps/simple-search', {
    query: input.query,
    hl: input.language || 'en',
    gl: input.country,
});
```

**Analysis:**
- ✓ `query` parameter is passed correctly
- ✓ `input.language` is correctly mapped to `hl` with default 'en'
- ✓ `input.country` is correctly mapped to `gl`
- ✗ `debug` parameter is never passed (even though API supports it)

**Expected Scrappa API Request:**
```
GET /maps/simple-search?query=pizza&hl=en&gl=us
```

**What Actor Sends:**
```
GET /maps/simple-search?query=pizza&hl=en&gl=us
```

**Verdict:** Parameter mapping is CORRECT in code, but input schema uses confusing names.

---

## RESPONSE OBJECT ISSUE

### Line 49 in main.ts

```typescript
if (response.results && response.results.length > 0) {
```

**Expected Scrappa Response Structure:**
```json
{
  "items": [
    { "name": "Pizza Place", ... },
    { "name": "Another Pizza", ... }
  ]
}
```

**What Code Expects:**
```json
{
  "results": [
    { "name": "Pizza Place", ... }
  ]
}
```

**CRITICAL BUG:** The condition will ALWAYS be false because `response.results` doesn't exist. The array is named `items`, not `results`.

**Impact:** 
- Data is NOT pushed to Actor dataset (pushData line is never executed)
- Only the full response object is stored in KeyValueStore (line 55)
- Users will not see individual records in dataset

---

## VALIDATION RULE COMPARISON

### Language (hl) Parameter

**Scrappa Regex:** `^[a-zA-Z]{2}(-[a-zA-Z]{2})?$`

Valid Examples:
- `en` ✓ (English)
- `de` ✓ (German)
- `fr` ✓ (French)
- `es` ✓ (Spanish)
- `en-US` ✓ (English - United States)
- `de-DE` ✓ (German - Germany)
- `pt-BR` ✓ (Portuguese - Brazil)
- `zh-CN` ✓ (Chinese - China)

Invalid Examples (would fail regex):
- `english` ✗ (too long)
- `e` ✗ (too short)
- `en-USA` ✗ (region part too long)
- `en_US` ✗ (underscore not hyphen)

**Apify Validation:** maxLength 5

This accepts:
- `en` ✓
- `de` ✓
- `en-US` ✓
- `en-USA` ✗ (correctly rejected, 6 chars)
- `hello` ✗ (not a valid language code, but passes validation)
- `12345` ✗ (numbers, but passes validation!)

**Issue:** The Apify validation is WAY too permissive. It accepts numbers, random strings, and 5-char garbage like "abcde" or "12345".

---

### Region (gl) Parameter

**Scrappa Regex:** `^[a-zA-Z]{2}$`

Valid Examples:
- `us` ✓ (United States)
- `de` ✓ (Germany)
- `uk` ✓ (United Kingdom)
- `jp` ✓ (Japan)
- `fr` ✓ (France)
- `br` ✓ (Brazil)
- `AU` ✓ (Australia - case insensitive)

Invalid Examples (would fail regex):
- `usa` ✗ (too long)
- `u` ✗ (too short)
- `US1` ✗ (contains number)
- `us-` ✗ (contains dash)

**Apify Validation:** maxLength 2

This accepts:
- `us` ✓
- `uk` ✓
- `de` ✓
- `12` ✗ (numbers, but passes validation!)
- `!@` ✗ (special chars, but passes validation!)

**Issue:** The Apify validation accepts any 2-character string, including numbers and special characters.

---

## SUMMARY TABLE

| Check | Scrappa | Actor | Status |
|-------|---------|-------|--------|
| query required | ✓ | ✓ | MATCH |
| hl parameter exists | ✓ | (as language) | ✗ NAME MISMATCH |
| hl default is 'en' | ✓ | ✓ | MATCH |
| hl validation | ✓ Regex | maxLength only | ✗ INCOMPLETE |
| gl parameter exists | ✓ | (as country) | ✗ NAME MISMATCH |
| gl validation | ✓ Regex | maxLength only | ✗ INCOMPLETE |
| debug parameter | ✓ Optional | ✗ Missing | ✗ MISSING |
| Response root is 'items' | ✓ | Expected 'results' | ✗ BUG |
| Captures 28 fields | ✓ | Only 8 fields | ✗ 71% MISSING |
| Complex objects handled | ✓ Photos, Hours | ✗ Not handled | ✗ MISSING |
| Type definitions complete | ✓ | ✓ Complete for 8 fields | ✗ INCOMPLETE |

