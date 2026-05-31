//! 分级配置系统。
//!
//! 优先级链(高 → 低):
//! 1. CLI 参数(由调用方在加载后手工覆盖)
//! 2. 环境变量(`DOIT_API__BASE_URL`,双下划线表嵌套)
//! 3. `--config <path>` 指定文件
//! 4. `./doit.toml`(项目级)
//! 5. `~/.config/doit/config.toml`(用户级)
//! 6. 内置默认值
//!
//! API key 等字段支持 `${ENV_VAR}` 引用,避免明文存储。

use std::path::{Path, PathBuf};

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

use crate::error::{DoitError, Result};

/// 顶层配置。各 section 独立合并,子键逐项覆盖。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api: ApiConfig,
    pub model: ModelConfig,
    pub output: OutputConfig,
    pub display: DisplayConfig,
    pub prompt: PromptConfig,
    pub locale: LocaleConfig,
}

/// API 端点与密钥。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// OpenAI 兼容端点。
    pub base_url: String,
    /// API 密钥。支持 `${ENV_VAR}` 引用环境变量。
    pub api_key: String,
}

/// 模型与采样参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    pub name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    /// 是否启用思维链(reasoning)。
    pub thinking: bool,
}

/// 命令输出截断阈值(exec / 命令捕获)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub truncate_chars: usize,
    pub truncate_lines: usize,
}

/// 交互式显隐开关。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub show_reasoning: bool,
    pub show_narration: bool,
    pub show_command_output: bool,
}

/// 系统提示覆盖。
///
/// 系统提示由 `template` 命令(命令注册表)动态生成。这里提供两种语义:
/// - `append_*`:在生成结果尾部追加项目自定义指令,注册表照常工作(常用)。
/// - `system_*`:完全覆盖,设置后直接使用该文本,跳过 template 生成(逃生舱)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_interactive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_interactive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_task: Option<String>,
}

/// 语言设置。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocaleConfig {
    /// 语言代码(`zh-CN` / `en`)。留空则自动检测。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com".to_string(),
            api_key: "${DEEPSEEK_API_KEY}".to_string(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "deepseek-v4-pro".to_string(),
            temperature: 0.7,
            max_tokens: 8192,
            thinking: true,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            truncate_chars: 2000,
            truncate_lines: 50,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_reasoning: true,
            show_narration: true,
            show_command_output: true,
        }
    }
}

impl Config {
    /// 用户级配置文件路径:`~/.config/doit/config.toml`。
    pub fn user_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "doit").map(|d| d.config_dir().join("config.toml"))
    }

    /// 项目级配置文件路径:`<cwd>/doit.toml`。
    pub fn project_path() -> PathBuf {
        PathBuf::from("doit.toml")
    }

    /// 按优先级链加载配置(不含 CLI 参数,由调用方在返回后覆盖)。
    ///
    /// `explicit` 为 `--config <path>` 指定的文件(可选)。
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let mut fig = Figment::from(Serialized::defaults(Config::default()));
        if let Some(user) = Self::user_path() {
            fig = fig.merge(Toml::file(user));
        }
        fig = fig.merge(Toml::file(Self::project_path()));
        if let Some(path) = explicit {
            fig = fig.merge(Toml::file(path));
        }
        fig = fig.merge(Env::prefixed("DOIT_").split("__"));

        let mut config: Config = fig
            .extract()
            .map_err(|e| DoitError::config(format!("failed to load config: {e}")))?;
        config.resolve_env();
        Ok(config)
    }

    /// 解析字符串字段中的 `${ENV_VAR}` 引用为对应环境变量值。
    fn resolve_env(&mut self) {
        self.api.api_key = expand_env(&self.api.api_key);
        self.api.base_url = expand_env(&self.api.base_url);
    }

    /// 序列化为 TOML 文本(用于 `doit config list`)。
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| DoitError::config(format!("failed to serialize config: {e}")))
    }
}

/// 将字符串中的 `${VAR}` 替换为环境变量值;未设置时替换为空串。
/// 不含 `${` 的字符串原样返回。
fn expand_env(input: &str) -> String {
    if !input.contains("${") {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let var = &after[..end];
            out.push_str(&std::env::var(var).unwrap_or_default());
            rest = &after[end + 1..];
        } else {
            // 无闭合 }, 原样保留剩余部分
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// 按点号路径读取单个配置值(`doit config get model.name`)。
/// 返回该键的 TOML 标量/子表的字符串表示。
pub fn get_value(config: &Config, key: &str) -> Result<String> {
    let value =
        toml::Value::try_from(config).map_err(|e| DoitError::config(format!("serialize: {e}")))?;
    let mut cur = &value;
    for part in key.split('.') {
        cur = cur
            .get(part)
            .ok_or_else(|| DoitError::config(format!("unknown config key: {key}")))?;
    }
    Ok(match cur {
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

/// 写入目标配置层。
pub enum Scope {
    User,
    Project,
}

/// 按点号路径设置单个配置值并写回对应层文件(`doit config set model.name ...`)。
/// 使用 toml_edit 无损保留注释与格式。值按目标键的类型推断(数字/布尔/字符串)。
pub fn set_value(scope: Scope, key: &str, value: &str) -> Result<PathBuf> {
    let path = match scope {
        Scope::User => Config::user_path()
            .ok_or_else(|| DoitError::config("cannot resolve user config dir"))?,
        Scope::Project => Config::project_path(),
    };

    let mut doc = if path.exists() {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| DoitError::io(e, format!("read {}", path.display())))?;
        text.parse::<toml_edit::DocumentMut>()
            .map_err(|e| DoitError::config(format!("parse {}: {e}", path.display())))?
    } else {
        toml_edit::DocumentMut::new()
    };

    let parts: Vec<&str> = key.split('.').collect();
    let (last, sections) = parts
        .split_last()
        .ok_or_else(|| DoitError::config("empty config key"))?;

    // 逐级进入/创建表
    let mut item = doc.as_item_mut();
    for sec in sections {
        item = &mut item[*sec];
        if item.is_none() {
            *item = toml_edit::Item::Table(toml_edit::Table::new());
        }
    }
    item[last] = toml_edit::value(parse_scalar(value));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DoitError::io(e, format!("create {}", parent.display())))?;
    }
    std::fs::write(&path, doc.to_string())
        .map_err(|e| DoitError::io(e, format!("write {}", path.display())))?;
    Ok(path)
}

/// 将字符串值推断为 TOML 标量:整数 / 浮点 / 布尔 / 字符串。
fn parse_scalar(value: &str) -> toml_edit::Value {
    if let Ok(b) = value.parse::<bool>() {
        return b.into();
    }
    if let Ok(i) = value.parse::<i64>() {
        return i.into();
    }
    if let Ok(f) = value.parse::<f64>() {
        return f.into();
    }
    value.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.model.name, "deepseek-v4-pro");
        assert_eq!(c.output.truncate_chars, 2000);
        assert!(c.display.show_reasoning);
    }

    #[test]
    fn expand_env_resolves_vars() {
        unsafe { std::env::set_var("DOIT_TEST_KEY", "secret123") };
        assert_eq!(expand_env("${DOIT_TEST_KEY}"), "secret123");
        assert_eq!(expand_env("plain"), "plain");
        assert_eq!(
            expand_env("pre-${DOIT_TEST_KEY}-post"),
            "pre-secret123-post"
        );
    }

    #[test]
    fn get_value_dotted() {
        let c = Config::default();
        assert_eq!(get_value(&c, "model.name").unwrap(), "deepseek-v4-pro");
        assert_eq!(get_value(&c, "output.truncate_lines").unwrap(), "50");
        assert!(get_value(&c, "model.nope").is_err());
    }

    #[test]
    fn parse_scalar_infers_type() {
        assert!(parse_scalar("true").is_bool());
        assert!(parse_scalar("42").is_integer());
        assert!(parse_scalar("0.5").is_float());
        assert!(parse_scalar("hello").is_str());
    }
}
