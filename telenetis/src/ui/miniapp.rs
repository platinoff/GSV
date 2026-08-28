//! Telegram Mini App native-layer helpers (band 215, plan P1).
//!
//! The browser-side Telegram WebApp SDK exposes `Telegram.WebApp.platform`,
//! `themeParams`, `BackButton`, `HapticFeedback` and `viewportStableHeight`
//! directly, so those are wired in `static/app.js`. This module holds the
//! *server-testable* logic so the layer is exercised in Rust: platform
//! classification (drives a platform body class), a safe CSS-custom-property
//! builder (feeds `--tg-theme-*` variables without injection), and the i18n
//! string table handed to the client so UI text is never hardcoded in JS.

/// Platform as reported by `Telegram.WebApp.platform`. Mirrors the Telegram
/// values: `ios`, `android`, `macos`, `windows`, `linux`, plus `web`/`unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Android,
    Macos,
    Windows,
    Linux,
    Web,
    Unknown,
}

impl Platform {
    /// Map a raw Telegram `platform` string. Unknown/blank values resolve to
    /// [`Platform::Unknown`] (safe default — no branch, no safe-area quirk).
    pub fn classify(raw: &str) -> Platform {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ios" | "iphone" | "ipad" => Platform::Ios,
            "android" => Platform::Android,
            "macos" | "mac" => Platform::Macos,
            "windows" | "win32" => Platform::Windows,
            "linux" => Platform::Linux,
            "web" | "tdesktop" => Platform::Web,
            _ => Platform::Unknown,
        }
    }

    /// CSS class added to `<body>` so styles can branch per platform + safe-area.
    pub fn body_class(self) -> &'static str {
        match self {
            Platform::Ios => "platform-ios",
            Platform::Android => "platform-android",
            Platform::Macos => "platform-macos",
            Platform::Windows => "platform-windows",
            Platform::Linux => "platform-linux",
            Platform::Web => "platform-web",
            Platform::Unknown => "platform-unknown",
        }
    }

    /// Whether the platform needs the safe-area insets handled (iOS/Android
    /// notch + gesture bars). Desktop/web do not.
    pub fn needs_safe_area(self) -> bool {
        matches!(self, Platform::Ios | Platform::Android)
    }
}

/// A CSS custom property — `--name: value`. Values are sanitized so a
/// hostile `initData`/theme value cannot break out of the declaration list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeVar {
    name: String,
    value: String,
}

impl ThemeVar {
    /// Build a custom property from a Telegram theme key (e.g. `bg_color`) by
    /// prefixing with `--tg-theme-`. Characters outside `[a-zA-Z0-9_-]` are
    /// dropped to keep the property name stable and non-injectible.
    pub fn tg(key: &str, value: &str) -> ThemeVar {
        let name: String = key
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        ThemeVar {
            name,
            value: safe_css_value(value),
        }
    }

    pub fn as_css(&self) -> String {
        format!("--tg-theme-{}: {};", self.name, self.value)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Allow only characters that are legal *inside* a CSS value string: hex color
/// digits, `#`, `,`, spaces, and alpha. Anything else is dropped.
fn safe_css_value(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            c.is_ascii_alphabetic()
                || c.is_ascii_digit()
                || matches!(c, '#' | ',' | ' ' | '.' | '%' | '(' | ')')
        })
        .collect()
}

/// Language code resolution with a fallback chain. The Mini App keys strings
/// by `initDataUnsafe.user.language_code`; unsupported codes fall back to `en`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Uk,
    Ru,
}

impl Lang {
    pub fn parse(raw: &str) -> Lang {
        match raw.trim().to_ascii_lowercase().as_str() {
            "uk" | "ua" | "uk-ua" => Lang::Uk,
            "ru" | "ru-ru" => Lang::Ru,
            _ => Lang::En,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
        }
    }
}

/// String keys used by the Mini App UI. Keeping the table in Rust means the
/// client JS merely indexes by key — no hardcoded user-visible text in JS.
pub fn t(key: &str, lang: Lang) -> &'static str {
    match lang {
        Lang::En => match key {
            "app.title" => "Telenetis",
            "app.subtitle" => "Telegram Mini App for GSV Godfather Channel",
            "status.loading" => "Loading...",
            "status.online" => "Online",
            "status.offline" => "Offline",
            "status.tickets" => "Tickets",
            "status.workers" => "Workers",
            "nav.board" => "Board",
            "nav.flows" => "Flows",
            "nav.roles" => "Roles",
            "action.claim" => "Claim",
            "action.done" => "Done",
            "action.error" => "Error",
            _ => "",
        },
        Lang::Uk => match key {
            "app.title" => "Теленетис",
            "app.subtitle" => "Міні-застосунок Telegram для каналу Godfather GSV",
            "status.loading" => "Завантаження...",
            "status.online" => "Онлайн",
            "status.offline" => "Офлайн",
            "status.tickets" => "Тикети",
            "status.workers" => "Воркери",
            "nav.board" => "Дошка",
            "nav.flows" => "Потоки",
            "nav.roles" => "Ролі",
            "action.claim" => "Взяти",
            "action.done" => "Готово",
            "action.error" => "Помилка",
            _ => "",
        },
        Lang::Ru => match key {
            "app.title" => "Теленетис",
            "app.subtitle" => "Мини-приложение Telegram для канала Godfather GSV",
            "status.loading" => "Загрузка...",
            "status.online" => "Онлайн",
            "status.offline" => "Офлайн",
            "status.tickets" => "Тикеты",
            "status.workers" => "Воркеры",
            "nav.board" => "Доска",
            "nav.flows" => "Потоки",
            "nav.roles" => "Роли",
            "action.claim" => "Взять",
            "action.done" => "Готово",
            "action.error" => "Ошибка",
            _ => "",
        },
    }
}

/// All keys rendered for a language — used by `GET /api/mini-app/i18n`.
pub const I18N_KEYS: &[&str] = &[
    "app.title",
    "app.subtitle",
    "status.loading",
    "status.online",
    "status.offline",
    "status.tickets",
    "status.workers",
    "nav.board",
    "nav.flows",
    "nav.roles",
    "action.claim",
    "action.done",
    "action.error",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_platforms() {
        assert_eq!(Platform::classify("ios"), Platform::Ios);
        assert_eq!(Platform::classify("ANDROID"), Platform::Android);
        assert_eq!(Platform::classify("macos"), Platform::Macos);
        assert_eq!(Platform::classify("windows"), Platform::Windows);
        assert_eq!(Platform::classify("linux"), Platform::Linux);
        assert_eq!(Platform::classify("web"), Platform::Web);
    }

    #[test]
    fn unknown_platform_is_safe_default() {
        let p = Platform::classify("quantum-browser");
        assert_eq!(p, Platform::Unknown);
        assert_eq!(p.body_class(), "platform-unknown");
        assert!(!p.needs_safe_area());
    }

    #[test]
    fn mobile_platforms_need_safe_area() {
        assert!(Platform::Ios.needs_safe_area());
        assert!(Platform::Android.needs_safe_area());
        assert!(!Platform::Web.needs_safe_area());
        assert!(!Platform::Windows.needs_safe_area());
    }

    #[test]
    fn body_classes_are_distinct() {
        let cs: Vec<&str> = ["ios", "android", "macos", "windows", "linux", "web", "??"]
            .into_iter()
            .map(|p| Platform::classify(p).body_class())
            .collect();
        assert!(cs.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn theme_var_builds_tg_css() {
        let v = ThemeVar::tg("bg_color", "#1c2733");
        assert_eq!(v.as_css(), "--tg-theme-bg_color: #1c2733;");
        assert_eq!(v.value(), "#1c2733");
    }

    #[test]
    fn theme_var_drops_hostile_value_characters() {
        let v = ThemeVar::tg("button_text_color", "red; color: rgb(0,0,0);--x:1");
        let out = v.as_css();
        // Payload `;`/`:` are stripped, so the value cannot be broken out of
        // or augmented. as_css() itself appends exactly one trailing `;`.
        assert!(v.value().contains("rgb(0,0,0)"));
        assert!(!v.value().contains(';'));
        assert!(!v.value().contains(':'));
        assert_eq!(out.matches(';').count(), 1);
        assert!(out.starts_with("--tg-theme-button_text_color: "));
    }

    #[test]
    fn language_falls_back_to_english() {
        assert_eq!(t("status.online", Lang::parse("en")), "Online");
        assert_eq!(t("status.online", Lang::parse("uk")), "Онлайн");
        assert_eq!(t("status.online", Lang::parse("ru")), "Онлайн");
        assert_eq!(t("status.online", Lang::parse("xx-YY")), "Online");
        assert_eq!(t("unknown.key", Lang::En), "");
    }

    #[test]
    fn language_codes_normalize() {
        assert_eq!(Lang::parse("UK"), Lang::Uk);
        assert_eq!(Lang::parse("ua"), Lang::Uk);
        assert_eq!(Lang::parse("ru-ru"), Lang::Ru);
        assert_eq!(Lang::parse("en-us"), Lang::En);
    }
}
