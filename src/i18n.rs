//! Application messages are translated; request/response data stays unchanged.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Language {
    #[serde(rename = "en")]
    English = 1,
    #[serde(rename = "ja")]
    Japanese = 2,
    #[serde(rename = "ko")]
    Korean = 3,
    #[serde(rename = "fr")]
    French = 4,
    #[serde(rename = "de")]
    German = 5,
    #[serde(rename = "ru")]
    Russian = 6,
    #[default]
    #[serde(rename = "zh-CN", other)]
    Chinese = 0,
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

// Persist the user's choice, not the language detected on this particular machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum LanguagePreference {
    #[default]
    Automatic,
    Fixed(Language),
}

impl LanguagePreference {
    pub fn from_code(code: &str) -> Option<Self> {
        if code == "auto" {
            Some(Self::Automatic)
        } else {
            Language::from_code(code).map(Self::Fixed)
        }
    }

    pub fn resolve(self) -> Language {
        match self {
            Self::Fixed(language) => language,
            Self::Automatic => self.resolve_locales(sys_locale::get_locales()),
        }
    }

    fn resolve_locales(self, locales: impl IntoIterator<Item = impl AsRef<str>>) -> Language {
        match self {
            Self::Fixed(language) => language,
            Self::Automatic => locales
                .into_iter()
                .find_map(|locale| Language::from_locale(locale.as_ref()))
                .unwrap_or(Language::English),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => Language::current().message("Automatic"),
            Self::Fixed(language) => language.label(),
        }
    }
}

impl From<String> for LanguagePreference {
    fn from(code: String) -> Self {
        Self::from_code(&code).unwrap_or_default()
    }
}

impl From<LanguagePreference> for String {
    fn from(preference: LanguagePreference) -> Self {
        match preference {
            LanguagePreference::Automatic => "auto",
            LanguagePreference::Fixed(language) => language.code(),
        }
        .into()
    }
}

impl Language {
    pub const ALL: [Self; 7] = [
        Self::Chinese,
        Self::English,
        Self::Japanese,
        Self::Korean,
        Self::French,
        Self::German,
        Self::Russian,
    ];

    pub fn current() -> Self {
        Self::ALL
            .get(LANGUAGE.load(Ordering::Relaxed) as usize)
            .copied()
            .unwrap_or_default()
    }
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
    }
    fn from_locale(locale: &str) -> Option<Self> {
        let primary = locale.trim().split(['-', '_', '.', '@']).next()?;
        match primary.to_ascii_lowercase().as_str() {
            "zh" => Some(Self::Chinese),
            "en" => Some(Self::English),
            "ja" => Some(Self::Japanese),
            "ko" => Some(Self::Korean),
            "fr" => Some(Self::French),
            "de" => Some(Self::German),
            "ru" => Some(Self::Russian),
            _ => None,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::French => "fr",
            Self::German => "de",
            Self::Russian => "ru",
        }
    }
    pub fn activate(self) {
        LANGUAGE.store(self as u8, Ordering::Relaxed);
        gpui_kit::component::set_locale(self.code());
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Chinese => "简体中文",
            Self::English => "English",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Russian => "Русский",
        }
    }
    pub fn message(self, english: &str) -> &str {
        lookup(self, english).unwrap_or(english)
    }
    // Retains the two-argument plugin API; unknown plugin text keeps its original fallback.
    pub fn text<'a>(self, chinese: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::Chinese => chinese,
            _ => self.message(english),
        }
    }
}

macro_rules! tr {
    ($message:literal) => {
        $crate::i18n::Language::current().message($message)
    };
}
// Generated format arms keep Rust's compile-time checks in every language.
include!(concat!(env!("OUT_DIR"), "/translations.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_language_matches_regions_scripts_and_posix_locales() {
        for (locale, language) in [
            ("zh-Hans-CN", Language::Chinese),
            ("zh_TW.UTF-8", Language::Chinese),
            ("en-GB", Language::English),
            ("ja-JP", Language::Japanese),
            ("ko_KR.UTF-8", Language::Korean),
            ("fr-CA", Language::French),
            ("de_DE.UTF-8@euro", Language::German),
            (" RU-ru ", Language::Russian),
        ] {
            assert_eq!(
                LanguagePreference::Automatic.resolve_locales([locale]),
                language,
                "{locale}"
            );
        }
    }

    #[test]
    fn automatic_language_uses_first_supported_preference_or_english() {
        let automatic = LanguagePreference::Automatic;
        assert_eq!(
            automatic.resolve_locales(["es-ES", "ja-JP", "en-US"]),
            Language::Japanese
        );
        assert_eq!(
            automatic.resolve_locales(["en-US", "ja-JP"]),
            Language::English
        );
        assert_eq!(
            automatic.resolve_locales(["", "C", "POSIX", "C.UTF-8", "it-IT"]),
            Language::English
        );
        assert_eq!(
            automatic.resolve_locales(std::iter::empty::<&str>()),
            Language::English
        );
    }

    #[test]
    fn manual_language_wins_over_system_preferences() {
        assert_eq!(
            LanguagePreference::Fixed(Language::French).resolve_locales(["zh-CN", "en-US"]),
            Language::French
        );
    }

    #[test]
    fn stored_automatic_and_existing_manual_choices_roundtrip() {
        for (code, expected) in std::iter::once(("auto", LanguagePreference::Automatic)).chain(
            Language::ALL.map(|language| (language.code(), LanguagePreference::Fixed(language))),
        ) {
            let value = format!("\"{code}\"");
            let preference = serde_json::from_str::<LanguagePreference>(&value).unwrap();
            assert_eq!(preference, expected);
            assert_eq!(serde_json::to_string(&preference).unwrap(), value);
        }
        assert_eq!(
            serde_json::from_str::<LanguagePreference>("\"es\"").unwrap(),
            LanguagePreference::Automatic
        );
        assert_eq!(LanguagePreference::from_code("typo"), None);
    }

    #[test]
    fn all_language_preferences_round_trip_and_unknown_values_fall_back() {
        for language in Language::ALL {
            let json = serde_json::to_string(&language).unwrap();
            assert_eq!(json, format!("\"{}\"", language.code()));
            assert_eq!(serde_json::from_str::<Language>(&json).unwrap(), language);
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
        assert_eq!(
            serde_json::from_str::<Language>("\"es\"").unwrap(),
            Language::Chinese
        );
        assert_eq!(Language::from_code("es"), None);
    }
    #[test]
    fn every_language_has_localized_ui_and_preserves_unknown_plugin_text() {
        let expected = [
            "请求体",
            "Body",
            "リクエスト本文",
            "요청 본문",
            "Corps",
            "Anfragetext",
            "Тело запроса",
        ];
        for (language, label) in Language::ALL.into_iter().zip(expected) {
            assert_eq!(language.message("Body"), label);
            assert_eq!(language.message("custom {{data}}"), "custom {{data}}");
        }
    }
}
