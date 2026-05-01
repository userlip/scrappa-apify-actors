# Google Maps Search Actor - Issue Checklist

## Current Status

The issues below were investigated before the Google Maps Search actor was updated. As of the current code, the critical schema, response mapping, validation, debug, dataset view, and documentation issues are fixed. Verification:

- `npm run typecheck`
- `node --test test/*.test.mjs`

## CRITICAL ISSUES (Must Fix Immediately)

### Issue 1: Wrong Response Array Property Name
- **File:** `src/main.ts`
- **Line:** 49
- **Severity:** CRITICAL - Silent failure
- **Description:** Code checks for `response.results` but API returns `response.items`
- **Current Code:**
  ```typescript
  if (response.results && response.results.length > 0) {
      await Actor.pushData(response.results);
  }
  ```
- **Fix Required:**
  ```typescript
  if (response.items && response.items.length > 0) {
      await Actor.pushData(response.items);
  }
  ```
- **Impact:** Without this fix, no data is pushed to the dataset. Users see empty results.
- **Test:** Search for a common term (e.g., "pizza restaurants") and verify data appears in dataset
- **Status:** ✅ Fixed - `src/main.ts` now reads and pushes `response.items`.

---

### Issue 2: Incomplete Response Type Definition
- **File:** `src/main.ts`
- **Lines:** 10-20 (BusinessResult interface)
- **Severity:** CRITICAL - Missing 20+ fields
- **Description:** BusinessResult interface only defines 8 fields, API provides 28
- **Current Code:**
  ```typescript
  interface BusinessResult {
      name: string;
      rating?: number;
      review_count?: number;
      address: string;  // ✗ API returns "full_address", not "address"
      phone_numbers?: string[];
      website?: string;
      type?: string;
      business_id?: string;
      [key: string]: unknown;  // ✗ Defeats type safety
  }
  ```
- **Missing Fields:**
  - Location: latitude, longitude, full_address, district, timezone
  - IDs: place_id, google_mid
  - Business Info: price_level, price_level_text, domain, subtypes, owner_id, owner_name, owner_link, order_link
  - Descriptions: short_description, full_description
  - Media: photos_sample (array)
  - Hours: opening_hours (array)
  - Status: current_status

- **Fix Required:** Expand interface to include all 28 response fields
- **Status:** ✅ Fixed - response item types now cover the Scrappa output fields in `src/output-aliases.ts`, including photos and opening hours.

---

### Issue 3: Incorrect Response Structure Mapping
- **File:** `src/main.ts`
- **Lines:** 22-25 (GoogleMapsSearchResponse interface)
- **Severity:** HIGH - Type mismatch
- **Description:** Interface expects `results` but API returns `items`
- **Current Code:**
  ```typescript
  interface GoogleMapsSearchResponse {
      results?: BusinessResult[];  // ✗ Should be "items"
      [key: string]: unknown;
  }
  ```
- **Fix Required:**
  ```typescript
  interface GoogleMapsSearchResponse {
      items: BusinessResult[];
      [key: string]: unknown;
  }
  ```
- **Status:** ✅ Fixed - `GoogleMapsSearchResponse` now exposes `items`.

---

## HIGH PRIORITY ISSUES

### Issue 4: Wrong Parameter Names in Input Schema
- **File:** `.actor/input_schema.json`
- **Lines:** Properties section
- **Severity:** HIGH - User confusion
- **Description:** Schema uses `language` and `country` but API expects `hl` and `gl`
- **Current Code:**
  ```json
  "properties": {
    "language": {
      "title": "Language",
      "type": "string",
      "description": "Language code (e.g., 'en', 'de', 'es')",
      "default": "en",
      "maxLength": 5
    },
    "country": {
      "title": "Country Code",
      "type": "string",
      "description": "Two-letter country code (e.g., 'us', 'uk', 'de')",
      "maxLength": 2
    }
  }
  ```
- **Fix Option A - Rename to match API:**
  ```json
  "hl": { "title": "Language", ... },
  "gl": { "title": "Country/Region", ... }
  ```
- **Fix Option B - Keep names but add documentation:**
  Keep current names but add note: "Sent as `hl` and `gl` parameters to the API"
- **Recommendation:** Option A (rename to match API for clarity)
- **Status:** ✅ Fixed - the input schema now uses `hl` and `gl`.

---

### Issue 5: Missing Validation Patterns
- **File:** `.actor/input_schema.json`
- **Lines:** Properties section
- **Severity:** HIGH - No format enforcement
- **Description:** Language and country parameters have no regex validation
- **Current:** Uses only `maxLength` constraints
- **Fix Required - Add pattern validation:**
  ```json
  {
    "language": {
      "type": "string",
      "pattern": "^[a-zA-Z]{2}(-[a-zA-Z]{2})?$",
      "title": "Language Code",
      "description": "ISO 639-1 code (e.g., 'en', 'de', 'en-US')",
      "default": "en",
      "maxLength": 5
    },
    "country": {
      "type": "string",
      "pattern": "^[a-zA-Z]{2}$",
      "title": "Country Code",
      "description": "ISO 3166-1 alpha-2 code (e.g., 'us', 'de', 'uk')",
      "maxLength": 2
    }
  }
  ```
- **Current Behavior:** Accepts "12345", "hello", "!@", etc.
- **Fixed Behavior:** Only accepts valid ISO codes
- **Status:** ✅ Fixed - `hl` and `gl` now have regex patterns matching the API format.

---

### Issue 6: Missing Debug Parameter
- **File:** `.actor/input_schema.json`
- **Lines:** Properties section
- **Severity:** MEDIUM - Inconsistency
- **Description:** Scrappa API supports optional `debug` parameter, but actor doesn't expose it
- **Fix Required:**
  ```json
  {
    "properties": {
      "debug": {
        "type": "boolean",
        "title": "Debug Mode",
        "description": "Enable debug logging (admin only)",
        "default": false
      }
    }
  }
  ```
- **Note:** May not be used in actor if not needed, but should be in schema for API consistency
- **Status:** ✅ Fixed - `debug` is exposed in the input schema and forwarded by `buildSearchParams`.

---

### Issue 7: Incorrect Dataset View Configuration
- **File:** `.actor/actor.json`
- **Lines:** 21 (transformation fields)
- **Severity:** HIGH - Field mismatch
- **Description:** Dataset view references wrong field names
- **Current Code:**
  ```json
  "transformation": {
    "fields": ["name", "rating", "review_count", "address", "phone", "website", "type"]
  }
  ```
- **Issues:**
  1. References "address" but API returns "full_address"
  2. References "phone" but API returns "phone_numbers"
  3. Missing high-value fields (latitude, longitude, place_id, business_id)

- **Fix Required:**
  ```json
  "transformation": {
    "fields": [
      "name",
      "type",
      "rating",
      "review_count",
      "full_address",
      "latitude",
      "longitude",
      "phone_numbers",
      "website",
      "business_id",
      "place_id",
      "timezone"
    ]
  }
  ```
- **Status:** ✅ Fixed - the dataset view now uses `full_address`, `phone_numbers`, location fields, and IDs.

---

## MEDIUM PRIORITY ISSUES

### Issue 8: Missing Complex Object Handling
- **File:** `src/main.ts`
- **Severity:** MEDIUM - Data not captured
- **Description:** No handling for nested objects (photos, opening hours)
- **Affected Fields:**
  - `photos_sample` - Array of photo objects with photo_id, photo_url, photo_url_large, video_thumbnail_url, latitude, longitude, type
  - `opening_hours` - Array of hours objects with day, hours, date, special_day

- **Current Behavior:** Includes them but with no special handling
- **Recommended Fix:** Either:
  1. Pass through as-is (current approach, acceptable)
  2. Flatten to separate columns (more work)
  3. Store as JSON strings (compatible with most systems)

- **Status:** ✅ Fixed - complex objects are preserved as nested dataset fields.

---

### Issue 9: Memory Allocation Review
- **File:** `.actor/actor.json`
- **Lines:** 12-14 (resources section)
- **Severity:** LOW - May need adjustment
- **Description:** 256MB allocated; may be insufficient for large result sets
- **Current Configuration:**
  ```json
  "resources": {
    "memoryMbytes": 256
  }
  ```
- **Recommendation:** Keep at 256MB for now; increase to 512MB if crashes occur with large datasets
- **Status:** ⬜ Monitor - no code change required unless production runs show memory pressure.

---

### Issue 10: Incomplete Documentation
- **File:** `.actor/README.md`
- **Severity:** MEDIUM - Users don't know what data is available
- **Description:** README doesn't list all 28 available response fields
- **Current:** Generic description, doesn't mention:
  - photos_sample availability
  - opening_hours structure
  - location data (latitude/longitude)
  - timezone information

- **Fix Required:** Expand output section with complete field list
- **Status:** ✅ Fixed - `.actor/README.md` now documents inputs, output fields, aliases, nested fields, and example output.

---

## SUMMARY

| Priority | Count | Current Status |
|----------|-------|----------------|
| Critical | 3 | Fixed |
| High | 4 | Fixed |
| Medium | 2 | Fixed |
| Low | 1 | Monitor memory usage |
| **Total** | **10** | **9 fixed, 1 monitor item** |

---

## TESTING CHECKLIST

After fixes are applied:

- [x] Fix Issue 1: Change `response.results` to `response.items`
- [x] Fix Issue 2: Expand response item interface to cover Scrappa fields
- [x] Fix Issue 3: Fix GoogleMapsSearchResponse interface
- [x] Fix Issue 4: Rename parameters or add clear documentation
- [x] Fix Issue 5: Add regex pattern validation
- [x] Fix Issue 6: Add debug parameter to schema
- [x] Fix Issue 7: Update dataset view field names
- [x] Fix Issue 8: Review complex object handling
- [ ] Monitor Issue 9: Check memory usage in production
- [x] Fix Issue 10: Expand documentation

### Manual Testing Steps

1. **Test basic search:**
   - Input: query="pizza restaurants", hl="en", gl="us"
   - Verify: Results appear in dataset (not empty)
   - Verify: All 8 core fields populated (name, rating, review_count, full_address, phone_numbers, website, type, business_id)

2. **Test optional fields:**
   - Verify: latitude/longitude present (20+ records)
   - Verify: place_id present (20+ records)
   - Verify: photos_sample array included
   - Verify: opening_hours array included

3. **Test parameter validation:**
   - Try hl="12345" - should fail
   - Try gl="123" - should fail (3 chars)
   - Try hl="en-US" - should succeed
   - Try gl="de" - should succeed

4. **Test dataset view:**
   - Verify fields display correctly
   - Verify no "address" or "phone" fields (should be full_address, phone_numbers)
   - Verify latitude/longitude are displayed

---

## FILES TO MODIFY

1. `/Users/marindelija/Documents/Development/scrappa-apify-actors/actors/google-maps-search/src/main.ts`
2. `/Users/marindelija/Documents/Development/scrappa-apify-actors/actors/google-maps-search/.actor/input_schema.json`
3. `/Users/marindelija/Documents/Development/scrappa-apify-actors/actors/google-maps-search/.actor/actor.json`
4. `/Users/marindelija/Documents/Development/scrappa-apify-actors/actors/google-maps-search/.actor/README.md`
