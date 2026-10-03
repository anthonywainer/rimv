use serde::Serialize;

/// Provider-independent language metadata passed to the native selector.
/// `id` remains the exact identifier accepted by the selected provider; the
/// locale is only used for display and region-sensitive search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LanguagePresentation {
    pub id: String,
    pub locale: String,
    pub canonical_locale: String,
    pub language_name: String,
    pub region_code: Option<String>,
    pub region_name: Option<String>,
    pub flag: String,
    pub search_terms: Vec<String>,
    pub supports_auto_detect: bool,
}

pub(crate) fn present_languages(
    ids: &[String],
    supports_auto_detect: bool,
) -> Vec<LanguagePresentation> {
    ids.iter()
        .filter(|id| !id.eq_ignore_ascii_case("auto"))
        .map(|id| present_language(id, supports_auto_detect))
        .collect()
}

pub(crate) fn present_language(id: &str, supports_auto_detect: bool) -> LanguagePresentation {
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
        normalized.clone()
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
        supports_auto_detect,
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
    fn language_level_capabilities_get_shared_canonical_presentation() {
        let expected = [
            ("en", "English", "en-US", "🇺🇸"),
            ("es", "Spanish", "es-ES", "🇪🇸"),
            ("fr", "French", "fr-FR", "🇫🇷"),
            ("de", "German", "de-DE", "🇩🇪"),
            ("it", "Italian", "it-IT", "🇮🇹"),
            ("pt", "Portuguese", "pt-PT", "🇵🇹"),
        ];
        for (id, name, locale, flag) in expected {
            let presentation = present_language(id, true);
            assert_eq!(presentation.id, id);
            assert_eq!(presentation.language_name, name);
            assert_eq!(presentation.locale, locale);
            assert_eq!(presentation.flag, flag);
            assert!(presentation.supports_auto_detect);
        }
    }

    #[test]
    fn regional_provider_locale_is_preserved_and_base_language_does_not_leak_it() {
        let native = present_language("en-GB", false);
        let parakeet = present_language("en", true);
        assert_eq!(native.id, "en-GB");
        assert_eq!(native.locale, "en-GB");
        assert_eq!(native.canonical_locale, "en-US");
        assert_eq!(native.flag, "🇬🇧");
        assert_eq!(parakeet.id, "en");
        assert_eq!(parakeet.locale, "en-US");
        assert_eq!(parakeet.canonical_locale, "en-US");
        assert_eq!(parakeet.flag, "🇺🇸");
        assert!(
            parakeet
                .search_terms
                .iter()
                .any(|term| term == "United States")
        );
        let latin_american_spanish = present_language("es-419", false);
        assert_eq!(
            latin_american_spanish.region_name.as_deref(),
            Some("Latin America")
        );
        assert_eq!(latin_american_spanish.flag, "🌐");
    }
}
