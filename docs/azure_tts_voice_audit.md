# Azure TTS Voice Audit

**Date:** 2025-10-19T13:33:35Z  
**Objective:** Verify every Azure TTS voice in `backend/src/tts_service.rs` against Azure's official sources

## Current Voice Mappings in Code

From `backend/src/tts_service.rs` (lines 44-71):

```rust
fn map_language_to_voice(language_code: &str) -> &'static str {
    match language_code {
        // Spanish dialects - THESE NEED VERIFICATION
        "es-MX" => "es-MX-DaliaNeural",      // TODO: Verify
        "es-ES" => "es-ES-ElviraNeural",     // TODO: Verify  
        "es-AR" => "es-AR-ElenaNeural",      // TODO: Verify
        "es-CU" => "es-CU-BelkysNeural",     // TODO: Verify
        "es-CL" => "es-CL-CatalinaNeural",   // TODO: Verify
        "es-CO" => "es-CO-SalomeNeural",     // TODO: Verify

        // Arabic dialects - THESE NEED VERIFICATION
        "ar-EG" => "ar-EG-SalmaNeural",      // TODO: Verify
        "ar-LB" => "ar-LB-LaylaNeural",      // TODO: Verify
        "ar-SA" => "ar-SA-ZariyahNeural",    // TODO: Verify
        "ar-MA" => "ar-MA-MounaNeural",      // TODO: Verify
        "ar-IQ" => "ar-IQ-RanaNeural",       // TODO: Verify

        // French dialects - THESE NEED VERIFICATION
        "fr-CA" => "fr-CA-SylvieNeural",     // TODO: Verify
        "fr-FR" => "fr-FR-DeniseNeural",     // TODO: Verify
        "fr-CH" => "fr-CH-ArianeNeural",     // TODO: Verify
        "fr-BE" => "fr-BE-CharlineNeural",   // TODO: Verify
        "fr-CI" => "fr-CI-AkanNeural",       // TODO: Verify

        // Fallback to known working voice
        _ => "en-US-AriaNeural",               // This is confirmed to exist
    }
}
```

## Supported Dialects (from WARP.md)

**Spanish** (6): Mexican (es-MX), Castilian (es-ES), Argentinian (es-AR), Caribbean (es-CU/PR/DO), Chilean (es-CL), Colombian (es-CO)

**Arabic** (5): Egyptian (ar-EG), Levantine (ar-LB/SY/JO/PS), Gulf (ar-SA/AE/KW/QA/BH/OM), Maghrebi (ar-MA/DZ/TN/LY), Iraqi (ar-IQ)

**French** (5): Quebecois (fr-CA), Parisian (fr-FR), Swiss (fr-CH), Belgian (fr-BE), African (fr-CI/SN/CM)

## Azure Official Documentation Sources

### Language Support Page
- URL: https://learn.microsoft.com/en-us/azure/ai-services/speech-service/language-support?tabs=tts
- Last accessed: 2025-10-19T13:33:35Z
- **Status**: Confirms language codes are supported, but doesn't list specific voice names

### Voice Examples Found
From https://learn.microsoft.com/en-us/azure/ai-services/speech-service/speech-synthesis-markup-voice#use-voice-elements:
- `en-US-AvaMultilingualNeural`
- `en-US-AndrewMultilingualNeural`
- `en-US-AriaNeural` (mentioned in fallback - CONFIRMED)

## Investigation Progress

## VERIFICATION RESULTS

Date: 2025-10-19T13:48:14Z
Source: Azure voices list API data from azure_vocies.json

### ✅ SPANISH VOICES - VERIFIED
- **es-MX-DaliaNeural** ✅ CORRECT - Found in API
- **es-ES-ElviraNeural** ✅ CORRECT - Found in API  
- **es-AR-ElenaNeural** ✅ CORRECT - Found in API
- **es-CU-BelkysNeural** ✅ CORRECT - Found in API
- **es-CL-CatalinaNeural** ✅ CORRECT - Found in API
- **es-CO-SalomeNeural** ✅ CORRECT - Found in API

### ✅ ARABIC VOICES - VERIFIED
- **ar-EG-SalmaNeural** ✅ CORRECT - Found in API (line 169)
- **ar-LB-LaylaNeural** ✅ CORRECT - Found in API (line 295)
- **ar-SA-ZariyahNeural** ✅ CORRECT - Found in API (line 446)
- **ar-MA-MounaNeural** ✅ CORRECT - Found in API (line 347)
- **ar-IQ-RanaNeural** ✅ CORRECT - Found in API (line 206)

### ⚠️ FRENCH VOICES - MIXED RESULTS
- **fr-CA-SylvieNeural** ✅ CORRECT - Found in API (line 11382)
- **fr-FR-DeniseNeural** ✅ CORRECT - Found in API (line 11469)
- **fr-CH-ArianeNeural** ✅ CORRECT - Found in API (line 11443)
- **fr-BE-CharlineNeural** ✅ CORRECT - Found in API (line 11356)
- **fr-CI-AkanNeural** ❌ **DOES NOT EXIST** - NOT FOUND in Azure API

## Key Findings

### ❌ PROBLEM: Documentation Search Failed
After extensive searching of official Microsoft documentation, I could NOT find a comprehensive public list of exact neural voice names (ShortName format) for the specific dialects needed.

### ✅ SOLUTION: Use Azure Voices List API
From the REST API documentation (https://learn.microsoft.com/en-us/azure/ai-services/speech-service/rest-text-to-speech#get-a-list-of-voices), Azure provides an authoritative endpoint:

```
GET https://{region}.tts.speech.microsoft.com/cognitiveservices/voices/list
Headers: Ocp-Apim-Subscription-Key: {subscription_key}
```

This returns JSON with EXACT voice names in ShortName format (e.g., "es-MX-DaliaNeural").

## CRITICAL STATUS

**🚨 CANNOT PROCEED WITHOUT AZURE CREDENTIALS**

To verify the 16 voice names in the code, I need:
1. AZURE_SPEECH_REGION (e.g., "eastus", "westus2")
2. AZURE_SPEECH_KEY (subscription key)

## Next Steps

1. ✅ **COMPLETED**: Extensive documentation search - no public comprehensive list found
2. ✅ **COMPLETED**: Found official API endpoint for authoritative voice list
3. 🔴 **BLOCKED**: Need Azure credentials to call voices list API
4. ⏳ **PENDING**: Call API to get complete voice inventory for target region
5. ⏳ **PENDING**: Cross-reference code voice names against API response
6. ⏳ **PENDING**: Update code with verified voice names
7. ⏳ **PENDING**: Create integration test to prevent future regressions

## Evidence of Extensive Search

### URLs Searched (All Unsuccessful for Voice Names):
- https://learn.microsoft.com/en-us/azure/ai-services/speech-service/language-support?tabs=tts
- https://learn.microsoft.com/en-us/azure/ai-services/speech-service/speech-synthesis-markup-voice#use-voice-elements
- https://learn.microsoft.com/en-us/azure/ai-services/speech-service/rest-text-to-speech
- https://speech.microsoft.com/portal/voicegallery (page failed to load)
- Various Azure SDK samples and GitHub repositories
- Stack Overflow and community resources

### What Was Found:
- Language support confirmation (locales exist)
- API structure documentation
- Example voices (en-US-JennyNeural, en-US-AvaMultilingualNeural)
- **NO comprehensive list of dialect-specific neural voice names**

## DETAILED API EVIDENCE

### Spanish Voice Confirmations
```json
{
  "ShortName": "es-MX-DaliaNeural",
  "Locale": "es-MX", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "es-ES-ElviraNeural",
  "Locale": "es-ES", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "es-AR-ElenaNeural",
  "Locale": "es-AR", "Status": "GA", "VoiceType": "Neural" 
}
{
  "ShortName": "es-CU-BelkysNeural",
  "Locale": "es-CU", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "es-CL-CatalinaNeural",
  "Locale": "es-CL", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "es-CO-SalomeNeural",
  "Locale": "es-CO", "Status": "GA", "VoiceType": "Neural"
}
```

### Arabic Voice Confirmations
```json
{
  "ShortName": "ar-EG-SalmaNeural",
  "Locale": "ar-EG", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "ar-LB-LaylaNeural", 
  "Locale": "ar-LB", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "ar-SA-ZariyahNeural",
  "Locale": "ar-SA", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "ar-MA-MounaNeural",
  "Locale": "ar-MA", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "ar-IQ-RanaNeural",
  "Locale": "ar-IQ", "Status": "GA", "VoiceType": "Neural"
}
```

### French Voice Confirmations
```json
{
  "ShortName": "fr-CA-SylvieNeural",
  "Locale": "fr-CA", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "fr-FR-DeniseNeural",
  "Locale": "fr-FR", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "fr-CH-ArianeNeural",
  "Locale": "fr-CH", "Status": "GA", "VoiceType": "Neural"
}
{
  "ShortName": "fr-BE-CharlineNeural",
  "Locale": "fr-BE", "Status": "GA", "VoiceType": "Neural"
}
```

### ❌ fr-CI Voice - NOT FOUND
Searched entire azure_vocies.json file - **NO fr-CI locale exists in Azure's voice inventory**.

## COMPREHENSIVE COMPARISON TABLE

| Dialect | Code Voice | Status | API Evidence | Action Required |
|---------|------------|--------|--------------|----------------|
| es-MX | es-MX-DaliaNeural | ✅ CORRECT | Found line 10495 | None |
| es-ES | es-ES-ElviraNeural | ✅ CORRECT | Found line 9477 | None |
| es-AR | es-AR-ElenaNeural | ✅ CORRECT | Found line 9259 | None |
| es-CU | es-CU-BelkysNeural | ✅ CORRECT | Found line 9399 | None |
| es-CL | es-CL-CatalinaNeural | ✅ CORRECT | Found line 9321 | None |
| es-CO | es-CO-SalomeNeural | ✅ CORRECT | Found line 9347 | None |
| ar-EG | ar-EG-SalmaNeural | ✅ CORRECT | Found line 169 | None |
| ar-LB | ar-LB-LaylaNeural | ✅ CORRECT | Found line 295 | None |
| ar-SA | ar-SA-ZariyahNeural | ✅ CORRECT | Found line 446 | None |
| ar-MA | ar-MA-MounaNeural | ✅ CORRECT | Found line 347 | None |
| ar-IQ | ar-IQ-RanaNeural | ✅ CORRECT | Found line 206 | None |
| fr-CA | fr-CA-SylvieNeural | ✅ CORRECT | Found line 11382 | None |
| fr-FR | fr-FR-DeniseNeural | ✅ CORRECT | Found line 11469 | None |
| fr-CH | fr-CH-ArianeNeural | ✅ CORRECT | Found line 11443 | None |
| fr-BE | fr-BE-CharlineNeural | ✅ CORRECT | Found line 11356 | None |
| fr-CI | fr-CI-AkanNeural | ❌ **INVALID** | NOT FOUND in API | **REMOVE or REPLACE** |

## FINAL SUMMARY

### ✅ SUCCESS RATE: 15/16 voices (93.75%)
- **15 voices VERIFIED and CORRECT**
- **1 voice INVALID and must be fixed**

### Required Actions:
1. **CRITICAL**: Remove or replace `fr-CI-AkanNeural` - this voice does not exist in Azure's API
2. **OPTIONAL**: All other voices are correct and can remain unchanged

### Recommendations:
- Remove fr-CI from supported dialects OR
- Use fallback to a supported French voice for fr-CI locale OR 
- Find alternative dialect code that Azure supports

## CONCLUSION

**TASK COMPLETED SUCCESSFULLY** using Azure API data. Found critical issue: **fr-CI-AkanNeural does not exist** and will cause 400 errors in production.
