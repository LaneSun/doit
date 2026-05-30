pub fn detect_locale() -> &'static str {
    if let Ok(lang) = std::env::var("LANG") {
        let normalized = lang.to_lowercase().replace('-', "_");
        if normalized.starts_with("zh_cn") {
            return "zh-CN";
        }
    }
    "en"
}

/// 将配置中的语言代码归一化为受支持的静态字符串。
pub fn normalize_locale(lang: &str) -> &'static str {
    let normalized = lang.to_lowercase().replace('-', "_");
    if normalized.starts_with("zh_cn") {
        "zh-CN"
    } else {
        "en"
    }
}
