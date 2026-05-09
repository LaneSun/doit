pub fn detect_locale() -> &'static str {
    if let Ok(lang) = std::env::var("LANG") {
        let normalized = lang.to_lowercase().replace('-', "_");
        if normalized.starts_with("zh_cn") {
            return "zh-CN";
        }
    }
    "en"
}
