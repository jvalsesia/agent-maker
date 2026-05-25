//! F09 shared locale primitives used by settings, agents, and chat.
//!
//! v1 supports English and Brazilian Portuguese. Adding a locale means
//! extending [`SUPPORTED_LOCALES`], [`language_name`], and widening the
//! matching CHECK constraints in a follow-up migration.

/// BCP 47 locale codes the backend accepts for the UI locale and for a
/// specific (non-`auto`) per-agent response language.
pub const SUPPORTED_LOCALES: &[&str] = &["en", "pt-BR"];

/// Returns true when `locale` is one of [`SUPPORTED_LOCALES`].
pub fn is_supported(locale: &str) -> bool {
    SUPPORTED_LOCALES.contains(&locale)
}

/// Returns true when `value` is a valid per-agent `response_language`:
/// either `auto` (follow the UI locale) or a supported locale.
pub fn is_valid_response_language(value: &str) -> bool {
    value == "auto" || is_supported(value)
}

/// Human-readable language name for a locale, used to build the
/// "Respond in <language>" chat directive. Unknown locales map to English.
pub fn language_name(locale: &str) -> &'static str {
    match locale {
        "pt-BR" => "Brazilian Portuguese",
        _ => "English",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_set_matches_constraints() {
        assert!(is_supported("en"));
        assert!(is_supported("pt-BR"));
        assert!(!is_supported("pt"));
        assert!(!is_supported("xx-YY"));
    }

    #[test]
    fn response_language_allows_auto_and_supported() {
        assert!(is_valid_response_language("auto"));
        assert!(is_valid_response_language("pt-BR"));
        assert!(!is_valid_response_language("fr"));
        assert!(!is_valid_response_language(""));
    }

    #[test]
    fn language_name_falls_back_to_english() {
        assert_eq!(language_name("pt-BR"), "Brazilian Portuguese");
        assert_eq!(language_name("en"), "English");
        assert_eq!(language_name("unknown"), "English");
    }
}
