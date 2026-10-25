pub(super) fn is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('_', "-");
    matches!(
        key.as_str(),
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie"
    ) || [
        "secret",
        "password",
        "passwd",
        "token",
        "api-key",
        "apikey",
        "access-key",
        "private-key",
    ]
    .iter()
    .any(|name| key == *name || key.ends_with(&format!("-{name}")))
}

pub(super) fn visible_value<'a>(key: &str, value: &'a str, revealed: bool) -> &'a str {
    if is_sensitive(key) && !revealed {
        "••••••••"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_debug_headers_are_visible_without_a_toggle() {
        for key in [
            "appid",
            "ts",
            "sign",
            "x-appid",
            "x-timestamp",
            "x-sign",
            "x-client",
            "key-id",
        ] {
            assert_eq!(visible_value(key, "actual-value", false), "actual-value");
        }
    }

    #[test]
    fn only_sensitive_fields_require_reveal() {
        for key in [
            "secret",
            "x-secret",
            "password",
            "Authorization",
            "x_api_key",
            "access-token",
            "Cookie",
        ] {
            assert_eq!(visible_value(key, "credential", false), "••••••••");
            assert_eq!(visible_value(key, "credential", true), "credential");
        }
    }
}
