//! anyCode Onboarding & Configuration
//!
//! 引导式配置系统，类似 OpenClaw 的接入流程

use anycode_core::prelude::*;
use anycode_llm::build_llm_client;
use anycode_llm::ProviderConfig;
use anycode_llm::{zai_model_display_name, ZAI_DEFAULT_CODING_ENDPOINT, ZAI_MODEL_CATALOG};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use dirs::config_dir;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// z.ai 配置（历史遗留：这里曾叫 BigModel）
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiConfig {
    pub api_key: String,
    /// OpenAPI `model` id（与 anyCode `ZAI_MODEL_CATALOG` 对齐，可为任意目录内 id）
    pub model: String,
    pub base_url: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub success: bool,
    pub status: u16,
    pub message: String,
}

// ============================================================================
// Onboarding Wizard
// ============================================================================

pub struct OnboardingWizard {
    config_dir: PathBuf,
}

impl OnboardingWizard {
    pub fn new() -> Result<Self> {
        let config_dir = config_dir().unwrap_or_else(|| {
            let mut path = std::env::current_exe().unwrap();
            path.pop();
            path.push(".anycode");
            path
        });

        // 创建配置目录
        std::fs::create_dir_all(&config_dir)?;

        Ok(Self { config_dir })
    }

    pub async fn run_zai_onboarding(&self) -> Result<ZaiConfig> {
        println!();
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║         🇨🇳 z.ai 接入向导 - anyCode                          ║");
        println!("║                                                              ║");
        println!("║  让我们一步步配置 z.ai API                                  ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!();

        // 步骤 1: 欢迎
        self.show_welcome().await?;

        // 步骤 2: 说明 z.ai
        self.explain_zai().await?;

        // 步骤 3: 获取 API Key
        let api_key = self.get_api_key().await?;

        // 步骤 4: 选择模型
        let model_str = self.select_model().await?;

        // 步骤 5: 配置参数
        let (temperature, max_tokens) = self.configure_parameters().await?;

        // 步骤 6: 测试连接
        let config = ZaiConfig {
            api_key,
            model: model_str.clone(),
            base_url: ZAI_DEFAULT_CODING_ENDPOINT.to_string(),
            temperature,
            max_tokens,
        };

        self.test_connection(&config).await?;

        // 步骤 7: 保存配置
        self.save_config(&config).await?;

        // 步骤 8: 完成
        self.show_completion(model_str.as_str()).await?;

        Ok(config)
    }

    async fn show_welcome(&self) -> Result<()> {
        println!("👋 欢迎使用 anyCode + z.ai！");
        println!();
        println!("z.ai 提供 OpenAI 兼容的 LLM 接口，可用于编码与通用对话。");
        println!("通过这个向导，您可以轻松配置 z.ai API，让 anyCode 为您服务。");
        println!();

        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("准备好开始了吗？")
            .default(false)
            .interact()?;

        Ok(())
    }

    async fn explain_zai(&self) -> Result<()> {
        println!("📚 关于 z.ai:");
        println!();
        println!("  • Provider：z.ai");
        println!("  • 默认 endpoint：{}", ZAI_DEFAULT_CODING_ENDPOINT);
        println!();
        println!("🤖 可用模型:");
        for e in ZAI_MODEL_CATALOG.iter() {
            println!("  • {}：{}", e.display_name, e.description);
        }
        println!();
        println!("💡 提示：您需要先获取 z.ai 的 API Key");
        println!();

        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("已经准备好 API Key 了？")
            .default(false)
            .interact()?;

        Ok(())
    }

    async fn get_api_key(&self) -> Result<String> {
        println!("🔑 API Key 配置");
        println!();

        // 尝试从环境变量读取
        if let Ok(key) = std::env::var("ZAI_API_KEY") {
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("检测到环境变量 ZAI_API_KEY，是否使用？")
                .default(true)
                .interact()?
            {
                return Ok(key);
            }
        }

        println!("请输入您的 z.ai API Key：");
        println!("(提示：格式为 sk-xxxxxxxxxxxxxxxxxxxxx)");
        println!();

        let api_key = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("API Key")
            .validate_with(|input: &String| -> Result<(), String> {
                if input.is_empty() {
                    return Err("API Key 不能为空".to_string());
                }
                if !input.starts_with("sk-") {
                    return Err("API Key 格式不正确，应该以 sk- 开头".to_string());
                }
                if input.len() < 40 {
                    return Err("API Key 长度似乎不正确".to_string());
                }
                Ok(())
            })
            .interact()?;

        println!();

        Ok(api_key)
    }

    async fn select_model(&self) -> Result<String> {
        println!("🤖 选择模型");
        println!();

        let mut model_labels: Vec<String> = ZAI_MODEL_CATALOG
            .iter()
            .map(|e| e.wizard_line.to_string())
            .collect();
        model_labels.push("自定义 model id…".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("选择一个模型")
            .items(&model_labels)
            .default(0)
            .interact()?;

        if selection < ZAI_MODEL_CATALOG.len() {
            return Ok(ZAI_MODEL_CATALOG[selection].api_name.to_string());
        }
        let custom: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("model id")
            .interact_text()?;
        let t = custom.trim();
        if t.is_empty() {
            anyhow::bail!("model id 不能为空");
        }
        Ok(t.to_string())
    }

    async fn configure_parameters(&self) -> Result<(f32, u32)> {
        println!("⚙️  配置参数（可选）");
        println!();

        // Temperature
        let temperature = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("温度 (0.0-1.0，默认 0.7，直接回车使用默认值)")
            .allow_empty(true)
            .interact()?
            .trim()
            .to_string();

        let temperature = if temperature.is_empty() {
            0.7
        } else {
            temperature
                .parse::<f32>()
                .map_err(|_| anyhow::anyhow!("温度必须是 0.0-1.0 之间的数字"))?
        };

        // Max tokens
        let max_tokens = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("最大 Token 数 (默认 4096，直接回车使用默认值)")
            .allow_empty(true)
            .interact()?
            .trim()
            .to_string();

        let max_tokens = if max_tokens.is_empty() {
            4096
        } else {
            max_tokens
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("Token 数必须是正整数"))?
        };

        println!();
        println!(
            "✅ 配置完成：temperature={}, max_tokens={}",
            temperature, max_tokens
        );

        Ok((temperature, max_tokens))
    }

    async fn test_connection(&self, config: &ZaiConfig) -> Result<()> {
        println!("🔗 测试连接");
        println!();

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(ProgressStyle::with_template("{spinner} {wide_msg}")?);
        spinner.set_message("正在连接 z.ai API...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        let provider_cfg = ProviderConfig {
            provider: "z.ai".to_string(),
            api_key: config.api_key.clone(),
            base_url: Some(config.base_url.clone()),
            model: config.model.clone(),
            temperature: Some(config.temperature),
            max_tokens: Some(config.max_tokens),
            zai_tool_choice_first_turn: false,
        };
        let llm = build_llm_client(&provider_cfg).await?;
        let model_cfg = ModelConfig {
            provider: LLMProvider::Custom("z.ai".to_string()),
            model: provider_cfg.model.clone(),
            base_url: provider_cfg.base_url.clone(),
            temperature: provider_cfg.temperature,
            max_tokens: provider_cfg.max_tokens,
            api_key: None,
        };

        let result: Result<TestResult> = match llm
            .chat(
                vec![Message {
                    id: uuid::Uuid::new_v4(),
                    role: MessageRole::User,
                    content: MessageContent::Text("你好".to_string()),
                    timestamp: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                }],
                vec![],
                &model_cfg,
            )
            .await
        {
            Ok(_) => Ok(TestResult {
                success: true,
                status: 200,
                message: "连接成功！".to_string(),
            }),
            Err(e) => Ok(TestResult {
                success: false,
                status: 500,
                message: format!("连接失败: {}", e),
            }),
        };

        spinner.finish();

        match result {
            Ok(test_result) => {
                if test_result.success {
                    println!("✅ 连接成功！");
                    println!("   状态码: {}", test_result.status);
                } else {
                    println!("❌ 连接失败：{}", test_result.message);
                    return Err(anyhow::anyhow!("连接测试失败"));
                }
            }
            Err(e) => {
                println!("❌ 连接错误：{}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn save_config(&self, config: &ZaiConfig) -> Result<()> {
        println!("💾 保存配置");
        println!();

        let config_file = self.config_dir.join("zai.json");
        let config_json = serde_json::to_string_pretty(config)?;

        std::fs::write(&config_file, config_json)?;

        println!("✅ 配置已保存到：{}", config_file.display());
        println!("   您可以在 ~/.config/anycode/zai.json 查看或修改配置");

        Ok(())
    }

    async fn show_completion(&self, model: &str) -> Result<()> {
        println!();
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║                  🎉 配置完成！                              ║");
        println!("║                                                              ║");
        println!("║  您现在可以使用 z.ai 了！                                   ║");
        println!("║                                                              ║");
        println!("║  使用示例：                                                    ║");
        println!("║    anycode run --agent general-purpose \"用中文帮我分析代码\"  ║");
        println!("║                                                              ║");
        println!(
            "║  当前模型: {}                              ║",
            zai_model_display_name(model)
        );
        println!("║                                                              ║");
        println!("║  下次启动时，anyCode 会自动加载 z.ai 配置                   ║");
        println!("║                                                              ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!();

        Ok(())
    }

    // 加载已保存的配置
    pub fn load_config(&self) -> Result<Option<ZaiConfig>> {
        let new_file = self.config_dir.join("zai.json");
        let old_file = self.config_dir.join("bigmodel.json");

        let config_file = if new_file.exists() {
            new_file
        } else if old_file.exists() {
            old_file
        } else {
            return Ok(None);
        };

        let config_json = std::fs::read_to_string(&config_file)?;
        let config: ZaiConfig = serde_json::from_str(&config_json)?;

        println!("📖 已加载 z.ai 配置");
        Ok(Some(config))
    }

    // 检查配置是否存在
    pub fn has_config(&self) -> bool {
        self.config_dir.join("zai.json").exists() || self.config_dir.join("bigmodel.json").exists()
    }
}

// ============================================================================
// CLI 命令
// ============================================================================

pub async fn onboard_zai() -> Result<()> {
    let wizard = OnboardingWizard::new()?;
    wizard.run_zai_onboarding().await?;
    Ok(())
}

pub fn load_zai_config() -> Result<Option<ZaiConfig>> {
    let wizard = OnboardingWizard::new()?;
    wizard.load_config()
}

pub fn check_zai_configured() -> bool {
    if let Ok(wizard) = OnboardingWizard::new() {
        wizard.has_config()
    } else {
        false
    }
}
