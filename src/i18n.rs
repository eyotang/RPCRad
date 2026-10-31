//! The UI and background workers share the single workspace's language.
//! Translate application messages only; never translate request or response data.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Language {
    #[serde(rename = "en")]
    English = 1,
    #[default]
    #[serde(rename = "zh-CN", other)]
    Chinese = 0,
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

impl Language {
    pub fn current() -> Self {
        match LANGUAGE.load(Ordering::Relaxed) {
            1 => Self::English,
            _ => Self::Chinese,
        }
    }

    pub fn activate(self) {
        LANGUAGE.store(self as u8, Ordering::Relaxed);
        gpui_kit::component::set_locale(match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        });
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Chinese => "简体中文",
            Self::English => "English",
        }
    }

    pub fn text<'a>(self, chinese: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::Chinese => chinese,
            Self::English => english,
        }
    }
}

// Keep both formats as literals so Rust checks placeholders in both languages.
macro_rules! tr {
    ($zh:literal, $en:literal) => {
        $crate::i18n::Language::current().text($zh, $en)
    };
}
macro_rules! tr_format {
    ($zh:literal, $en:literal $(, $args:expr)* $(,)?) => {
        match $crate::i18n::Language::current() {
            $crate::i18n::Language::Chinese => format!($zh $(, $args)*),
            $crate::i18n::Language::English => format!($en $(, $args)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_preference_round_trips_and_unknown_values_fall_back() {
        for (language, code) in [(Language::Chinese, "zh-CN"), (Language::English, "en")] {
            let json = serde_json::to_string(&language).unwrap();
            assert_eq!(json, format!("\"{code}\""));
            assert_eq!(serde_json::from_str::<Language>(&json).unwrap(), language);
        }
        assert_eq!(
            serde_json::from_str::<Language>("\"fr\"").unwrap(),
            Language::Chinese
        );
    }

    #[test]
    fn text_selects_only_the_requested_language() {
        assert_eq!(Language::Chinese.text("请求体", "Body"), "请求体");
        assert_eq!(Language::English.text("请求体", "Body"), "Body");
    }
}
