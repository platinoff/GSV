pub fn mini_app_url(base_url: &str) -> String {
    format!("{}/app", base_url)
}

/// Telegram `startapp` deep-link: `https://t.me/{username}?startapp={param}`.
/// The Mini App receives `param` as `initDataUnsafe.start_param`; the /app
/// shell redirects to the matching page (`probe` → `/probe`), because a
/// t.me link always opens the default page. `param` is sanitized to
/// `[a-zA-Z0-9_-]` (Telegram's allowed set) so a caller can never inject a
/// query break-out.
pub fn startapp_link(username: &str, param: &str) -> String {
    let clean: String = param
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let param = if clean.is_empty() {
        "telenetis".to_string()
    } else {
        clean
    };
    format!("https://t.me/{username}?startapp={param}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startapp_link_carries_probe_param() {
        assert_eq!(
            startapp_link("gsv_bot", "probe"),
            "https://t.me/gsv_bot?startapp=probe"
        );
    }

    #[test]
    fn startapp_link_falls_back_and_sanitizes() {
        assert_eq!(
            startapp_link("gsv_bot", ""),
            "https://t.me/gsv_bot?startapp=telenetis"
        );
        assert_eq!(
            startapp_link("gsv_bot", "a/b?c=d"),
            "https://t.me/gsv_bot?startapp=abcd"
        );
    }
}
