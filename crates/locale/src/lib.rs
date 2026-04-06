//! Resolve UI locale: `zh-Hans` vs `en` from environment and system defaults.
//!
//! Priority (first hit wins):
//! 1. `ANYCODE_LANG`
//! 2. `LANGUAGE` (first segment, e.g. `zh_CN:en`)
//! 3. `LC_ALL`
//! 4. `LC_MESSAGES`
//! 5. `LANG`
//! 6. OS UI locale (`sys_locale` where available)
//! 7. Fallback: **English** (international open-source default; Chinese users on zh_* systems still get zh from step 5–6).

use std::env;
use std::sync::OnceLock;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppLocale {
    ZhHans,
    En,
}

impl AppLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            AppLocale::ZhHans => "zh",
            AppLocale::En => "en",
        }
    }

    /// Fluent / BCP47-style id for resource loading.
    pub fn fluent_id(self) -> &'static str {
        match self {
            AppLocale::ZhHans => "zh",
            AppLocale::En => "en",
        }
    }
}

fn first_nonempty_env(keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Ok(v) = env::var(k) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn first_language_segment(language: &str) -> String {
    language
        .split(':')
        .next()
        .unwrap_or(language)
        .trim()
        .to_string()
}

/// Normalize a locale string to [AppLocale].
pub fn normalize_locale_tag(tag: &str) -> AppLocale {
    let t = tag.trim().to_lowercase();
    if t.starts_with("zh") {
        return AppLocale::ZhHans;
    }
    if t.starts_with("en") {
        return AppLocale::En;
    }
    // C / POSIX → English
    if t == "c" || t.starts_with("c.") {
        return AppLocale::En;
    }
    // Other locales default to English for our two-language product
    AppLocale::En
}

fn from_env_chain() -> Option<AppLocale> {
    if let Some(v) = first_nonempty_env(&["ANYCODE_LANG"]) {
        return Some(normalize_locale_tag(&v));
    }
    if let Some(v) = first_nonempty_env(&["LANGUAGE"]) {
        let seg = first_language_segment(&v);
        return Some(normalize_locale_tag(&seg));
    }
    if let Some(v) = first_nonempty_env(&["LC_ALL", "LC_MESSAGES", "LANG"]) {
        // LC_* often like zh_CN.UTF-8 — take part before . or @
        let core = v
            .split('.')
            .next()
            .unwrap_or(&v)
            .split('@')
            .next()
            .unwrap_or(&v)
            .to_string();
        return Some(normalize_locale_tag(&core));
    }
    None
}

fn from_system() -> Option<AppLocale> {
    let s = sys_locale::get_locale()?;
    Some(normalize_locale_tag(&s))
}

static CACHED: OnceLock<AppLocale> = OnceLock::new();

/// Resolve locale once per process (cached). Call `resolve_locale_clear_cache` in tests only.
pub fn resolve_locale() -> AppLocale {
    *CACHED.get_or_init(|| {
        from_env_chain()
            .or_else(from_system)
            .unwrap_or(AppLocale::En)
    })
}

/// Uncached resolution (for tests and tooling that must not use process-wide cache).
pub fn resolve_locale_from_env() -> AppLocale {
    from_env_chain()
        .or_else(from_system)
        .unwrap_or(AppLocale::En)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Env tests must not run in parallel.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let mut saved: Vec<(String, Option<String>)> = Vec::new();
        for (k, v) in vars {
            let key = (*k).to_string();
            saved.push((key.clone(), env::var(&key).ok()));
            match v {
                Some(val) => env::set_var(&key, val),
                None => env::remove_var(&key),
            }
        }
        f();
        for (k, v) in saved {
            match v {
                Some(val) => env::set_var(&k, val),
                None => env::remove_var(&k),
            }
        }
    }

    #[test]
    fn anycode_lang_wins() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[("ANYCODE_LANG", Some("en")), ("LANG", Some("zh_CN.UTF-8"))],
            || assert_eq!(resolve_locale_from_env(), AppLocale::En),
        );
    }

    #[test]
    fn lang_zh_cn() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("ANYCODE_LANG", None),
                ("LANGUAGE", None),
                ("LC_ALL", None),
                ("LC_MESSAGES", None),
                ("LANG", Some("zh_CN.UTF-8")),
            ],
            || assert_eq!(resolve_locale_from_env(), AppLocale::ZhHans),
        );
    }

    #[test]
    fn language_first_segment() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("ANYCODE_LANG", None),
                ("LANGUAGE", Some("zh_TW:en_US")),
                ("LANG", Some("en_US")),
            ],
            || assert_eq!(resolve_locale_from_env(), AppLocale::ZhHans),
        );
    }

    #[test]
    fn lc_messages_used() {
        let _g = ENV_LOCK.lock().unwrap();
        with_env(
            &[
                ("ANYCODE_LANG", None),
                ("LANGUAGE", None),
                ("LC_ALL", None),
                ("LC_MESSAGES", Some("zh_CN.UTF-8")),
                ("LANG", Some("C.UTF-8")),
            ],
            || assert_eq!(resolve_locale_from_env(), AppLocale::ZhHans),
        );
    }

    #[test]
    fn normalize_en_us() {
        assert_eq!(normalize_locale_tag("en_US.UTF-8"), AppLocale::En);
    }
}
