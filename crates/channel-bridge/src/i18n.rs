//! Fluent-based UI strings (daemon / wizards / wx). Locale from [`anycode_locale::resolve_locale`].

use anycode_locale::AppLocale;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

thread_local! {
    static BUNDLE: RefCell<Option<FluentBundle<FluentResource>>> = const { RefCell::new(None) };
    /// 翻译结果缓存（thread_local，避免全局锁）
    static TRANSLATION_CACHE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// 全局翻译缓存统计（用于性能监控）
static TRANSLATION_STATS: Lazy<Mutex<TranslationStats>> =
    Lazy::new(|| Mutex::new(TranslationStats::new()));

#[derive(Default)]
struct TranslationStats {
    cache_hits: u64,
    cache_misses: u64,
    total_translations: u64,
}

impl TranslationStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_hit(&mut self) {
        self.cache_hits += 1;
        self.total_translations += 1;
    }

    fn record_miss(&mut self) {
        self.cache_misses += 1;
        self.total_translations += 1;
    }

    fn hit_rate(&self) -> f64 {
        if self.total_translations == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_translations as f64
        }
    }
}

fn build_bundle() -> FluentBundle<FluentResource> {
    let loc = anycode_locale::resolve_locale();
    let lang: unic_langid::LanguageIdentifier = match loc {
        AppLocale::ZhHans => "zh".parse().expect("langid zh"),
        AppLocale::En => "en".parse().expect("langid en"),
    };
    let cli = match loc {
        AppLocale::ZhHans => include_str!("../locales/zh/cli.ftl"),
        AppLocale::En => include_str!("../locales/en/cli.ftl"),
    };
    let mut bundle = FluentBundle::new(vec![lang]);
    let res = FluentResource::try_new(cli.to_string()).expect("FTL parse");
    bundle.add_resource(res).expect("FTL add_resource");
    bundle
}

fn with_bundle<R>(f: impl FnOnce(&FluentBundle<FluentResource>) -> R) -> R {
    BUNDLE.with(|cell| {
        let mut g = cell.borrow_mut();
        if g.is_none() {
            *g = Some(build_bundle());
        }
        f(g.as_ref().expect("bundle"))
    })
}

fn format_msg(
    bundle: &FluentBundle<FluentResource>,
    id: &str,
    args: Option<&FluentArgs>,
) -> String {
    let mut errs = vec![];
    let Some(msg) = bundle.get_message(id) else {
        return format!("[missing:{id}]");
    };
    let Some(value) = msg.value() else {
        return format!("[no-value:{id}]");
    };
    let cow: Cow<'_, str> = bundle.format_pattern(value, args, &mut errs);
    cow.into_owned()
}

pub fn tr(id: &str) -> String {
    // 尝试从缓存获取
    let cache_key = id.to_string();

    TRANSLATION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(&cache_key) {
            // 缓存命中
            if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
                stats.record_hit();
            }
            return cached.clone();
        }

        // 缓存未命中，执行翻译
        let result = with_bundle(|b| format_msg(b, id, None));

        // 只缓存常用的翻译（避免内存膨胀）
        if is_common_translation_id(id) {
            cache.insert(cache_key, result.clone());
        }

        if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
            stats.record_miss();
        }

        result
    })
}

#[allow(dead_code)]
pub fn tr_args(id: &str, args: &FluentArgs<'_>) -> String {
    // 对于带参数的翻译，生成不同的缓存键
    let args_str = args
        .iter()
        .map(|(k, v)| format!("{}={:?}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    let cache_key = format!("{}|{}", id, args_str);

    TRANSLATION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(&cache_key) {
            if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
                stats.record_hit();
            }
            return cached.clone();
        }

        let result = with_bundle(|b| format_msg(b, id, Some(args)));

        // 带参数的翻译也缓存
        if is_common_translation_id(id) {
            cache.insert(cache_key, result.clone());
        }

        if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
            stats.record_miss();
        }

        result
    })
}

/// 判断是否是常用的翻译 ID（用于缓存策略）
fn is_common_translation_id(id: &str) -> bool {
    // 常见的 UI 文本，值得缓存
    const COMMON_PREFIXES: &[&str] = &[
        "term-", "cli-", "status-", "error-", "warn-", "confirm-", "cancel-",
    ];

    // 长度适中的翻译 ID 更可能是常用翻译
    if id.len() > 30 || id.len() < 5 {
        return false;
    }

    // 检查是否是常用前缀
    COMMON_PREFIXES.iter().any(|&prefix| id.starts_with(prefix))
}

/// 清空翻译缓存（用于切换语言或释放内存）
#[allow(dead_code)]
pub fn clear_translation_cache() {
    TRANSLATION_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
}

/// 获取翻译缓存统计信息（用于性能监控）
#[allow(dead_code)]
pub fn get_translation_stats() -> (u64, u64, f64) {
    if let Ok(stats) = TRANSLATION_STATS.try_lock() {
        (stats.cache_hits, stats.cache_misses, stats.hit_rate())
    } else {
        (0, 0, 0.0)
    }
}
