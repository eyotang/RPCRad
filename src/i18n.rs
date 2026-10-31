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
