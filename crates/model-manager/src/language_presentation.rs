//! Shared display metadata for provider-supplied transcription language IDs.
//!
//! `id` is always preserved verbatim for provider selection. `locale` is a
//! presentation locale and may use a canonical region when the provider gives
//! a language-level identifier such as `en`.
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguagePresentation {
    pub id: String,
    pub locale: String,
    pub canonical_locale: String,
    pub language_name: String,
    pub region_code: Option<String>,
    pub region_name: Option<String>,
    pub flag: String,
    pub search_terms: Vec<String>,
}

pub fn present_languages(ids: &[String]) -> Vec<LanguagePresentation> {
    let mut seen = HashSet::new();
    ids.iter()
        .filter_map(|id| {
            if id.eq_ignore_ascii_case("auto") || !seen.insert(id.to_ascii_lowercase()) {
                None
            } else {
                Some(present_language(id))
            }
        })
        .collect()
}

pub fn present_language(id: &str) -> LanguagePresentation {
    let normalized = id.replace('_', "-");
    let mut parts = normalized.split('-');
    let language = parts.next().unwrap_or(&normalized).to_ascii_lowercase();
    let supplied_region = parts
        .find(|part| {
            *part == "419"
                || (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_alphabetic()))
        })
        .map(str::to_ascii_uppercase);
    let (language_name, canonical_locale) = language_metadata(&language);
    let region_code = supplied_region.or_else(|| {
        canonical_locale
            .split_once('-')
            .map(|(_, region)| region.to_owned())
    });
    let locale = if normalized.contains('-') {
        normalized
    } else {
        canonical_locale.to_owned()
    };
    let region_name = region_code
        .as_deref()
        .and_then(region_name)
        .map(str::to_owned);
    let flag = region_code
        .as_deref()
        .and_then(flag_for_region)
        .unwrap_or_else(|| "🌐".to_owned());
    let mut search_terms = vec![
        id.to_owned(),
        language.clone(),
        language_name.to_owned(),
        locale.clone(),
    ];
    if let Some(name) = region_name.as_ref() {
        search_terms.push(name.clone());
    }
    search_terms.sort_unstable_by_key(|term| term.to_ascii_lowercase());
    search_terms.dedup_by(|left, right| left.eq_ignore_ascii_case(right));

    LanguagePresentation {
        id: id.to_owned(),
        locale,
        canonical_locale: canonical_locale.to_owned(),
        language_name: language_name.to_owned(),
        region_code,
        region_name,
        flag,
        search_terms,
    }
}

fn language_metadata(language: &str) -> (&'static str, &'static str) {
    match language {
        "ar" => ("Arabic", "ar-SA"),
        "bg" => ("Bulgarian", "bg-BG"),
        "ca" => ("Catalan", "ca-ES"),
        "cs" => ("Czech", "cs-CZ"),
        "da" => ("Danish", "da-DK"),
        "de" => ("German", "de-DE"),
        "el" => ("Greek", "el-GR"),
        "en" => ("English", "en-US"),
        "es" => ("Spanish", "es-ES"),
        "et" => ("Estonian", "et-EE"),
        "fi" => ("Finnish", "fi-FI"),
        "fr" => ("French", "fr-FR"),
        "he" => ("Hebrew", "he-IL"),
        "hi" => ("Hindi", "hi-IN"),
        "hr" => ("Croatian", "hr-HR"),
        "hu" => ("Hungarian", "hu-HU"),
        "id" => ("Indonesian", "id-ID"),
        "it" => ("Italian", "it-IT"),
        "ja" => ("Japanese", "ja-JP"),
        "ko" => ("Korean", "ko-KR"),
        "lt" => ("Lithuanian", "lt-LT"),
        "lv" => ("Latvian", "lv-LV"),
        "ms" => ("Malay", "ms-MY"),
        "mt" => ("Maltese", "mt-MT"),
        "nb" => ("Norwegian Bokmål", "nb-NO"),
        "nl" => ("Dutch", "nl-NL"),
        "pl" => ("Polish", "pl-PL"),
        "pt" => ("Portuguese", "pt-PT"),
        "ro" => ("Romanian", "ro-RO"),
        "ru" => ("Russian", "ru-RU"),
        "sk" => ("Slovak", "sk-SK"),
        "sl" => ("Slovenian", "sl-SI"),
        "sv" => ("Swedish", "sv-SE"),
        "th" => ("Thai", "th-TH"),
        "tr" => ("Turkish", "tr-TR"),
        "uk" => ("Ukrainian", "uk-UA"),
        "vi" => ("Vietnamese", "vi-VN"),
        "wuu" => ("Wu Chinese", "wuu-CN"),
        "yue" => ("Cantonese", "yue-CN"),
        "zh" => ("Chinese", "zh-CN"),
        _ => ("Unknown language", "und"),
    }
}

fn region_name(region: &str) -> Option<&'static str> {
    Some(match region {
        "AE" => "United Arab Emirates",
        "AT" => "Austria",
        "AU" => "Australia",
        "BE" => "Belgium",
        "BR" => "Brazil",
        "CA" => "Canada",
        "CH" => "Switzerland",
        "CL" => "Chile",
        "CN" => "China",
        "CO" => "Colombia",
        "CZ" => "Czechia",
        "DE" => "Germany",
        "DK" => "Denmark",
        "ES" => "Spain",
        "FI" => "Finland",
        "FR" => "France",
        "GB" => "United Kingdom",
        "GR" => "Greece",
        "HK" => "Hong Kong",
        "HU" => "Hungary",
        "ID" => "Indonesia",
        "IE" => "Ireland",
        "IL" => "Israel",
        "IN" => "India",
        "IT" => "Italy",
        "JP" => "Japan",
        "KR" => "South Korea",
        "MY" => "Malaysia",
        "MX" => "Mexico",
        "NL" => "Netherlands",
        "NO" => "Norway",
        "NZ" => "New Zealand",
        "PH" => "Philippines",
        "PL" => "Poland",
        "PT" => "Portugal",
        "RO" => "Romania",
        "RU" => "Russia",
        "SA" => "Saudi Arabia",
        "SE" => "Sweden",
        "SG" => "Singapore",
        "TH" => "Thailand",
        "TW" => "Taiwan",
        "UA" => "Ukraine",
        "US" => "United States",
        "VN" => "Vietnam",
        "ZA" => "South Africa",
        "419" => "Latin America",
        _ => return None,
    })
}

fn flag_for_region(region: &str) -> Option<String> {
    if region.len() != 2 || !region.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return None;
    }
    region
        .bytes()
        .map(|byte| char::from_u32(0x1F1E6 + u32::from(byte - b'A')))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_level_ids_get_canonical_regions_without_changing_provider_ids() {
        for (id, name, locale, flag, region) in [
            ("en", "English", "en-US", "🇺🇸", "United States"),
            ("es", "Spanish", "es-ES", "🇪🇸", "Spain"),
            ("fr", "French", "fr-FR", "🇫🇷", "France"),
            ("de", "German", "de-DE", "🇩🇪", "Germany"),
            ("it", "Italian", "it-IT", "🇮🇹", "Italy"),
            ("pt", "Portuguese", "pt-PT", "🇵🇹", "Portugal"),
        ] {
            let presentation = present_language(id);
            assert_eq!(presentation.id, id);
            assert_eq!(presentation.language_name, name);
            assert_eq!(presentation.locale, locale);
            assert_eq!(presentation.region_name.as_deref(), Some(region));
            assert_eq!(presentation.flag, flag);
        }
    }

    #[test]
    fn native_bcp47_locales_keep_their_region() {
        for (id, name, flag) in [("en-GB", "United Kingdom", "🇬🇧"), ("es-ES", "Spain", "🇪🇸")]
        {
            let presentation = present_language(id);
            assert_eq!(presentation.id, id);
            assert_eq!(presentation.locale, id);
            assert_eq!(presentation.region_name.as_deref(), Some(name));
            assert_eq!(presentation.flag, flag);
        }
    }

    #[test]
    fn search_terms_include_region_and_duplicate_provider_ids_are_removed() {
        let ids = ["en".to_owned(), "EN".to_owned(), "en-GB".to_owned()];
        let presentations = present_languages(&ids);
        assert_eq!(presentations.len(), 2);
        assert_eq!(presentations[0].id, "en");
        assert_eq!(presentations[1].id, "en-GB");
        assert!(
            presentations[0]
                .search_terms
                .iter()
                .any(|term| term == "United States")
        );
        assert!(
            presentations[1]
                .search_terms
                .iter()
                .any(|term| term == "United Kingdom")
        );
    }
}
