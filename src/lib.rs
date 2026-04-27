use anyhow::{anyhow, bail, Result};
use log::error;
use serde::Deserialize;

pub mod widgets;

pub enum Model {
    Claude(String),
    LMStudio(String),
}

/// A status data object.
///
/// Contains information required by status widgets.
///
pub struct StatusData {
    pub model: Model,
    pub ctx_total: usize,
    pub ctx_used: usize,
}

impl StatusData {
    /// Returns the percentage of the context used: [0.0, 100.0]
    pub fn ctx_usage_pct(&self) -> f32 {
        if self.ctx_total == 0 {
            return 0.0;
        }
        (self.ctx_used as f32 / self.ctx_total as f32) * 100.0
    }
}

#[derive(Deserialize, Debug)]
struct ClaudeData {
    model: ClaudeDataModel,
    context_window: ClaudeDataCtx,
}

#[derive(Deserialize, Debug)]
struct ClaudeDataModel {
    display_name: String,
}

#[derive(Deserialize, Debug)]
struct ClaudeDataCtx {
    pub context_window_size: usize,
    pub current_usage: Option<ClaudeDataCtxUsage>,
}

impl ClaudeDataCtx {
    /// Returns the total number of input tokens used.
    fn tokens_used(&self) -> usize {
        match &self.current_usage {
            None => 0,
            Some(current_usage) => current_usage.input_total(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ClaudeDataCtxUsage {
    input_tokens: usize,
    cache_creation_input_tokens: usize,
    cache_read_input_tokens: usize,
}

impl ClaudeDataCtxUsage {
    /// Returns the total number of input tokens.
    pub fn input_total(&self) -> usize {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}

/// Collects status data from:
/// - stdin (passed by ClaudeCode)
/// - environment variables
/// - LLM API (optional)
pub fn collect_data() -> Result<StatusData> {
    // Read clade code data from stdin
    let claude_data: ClaudeData = {
        use serde_json;
        let stdin = std::io::stdin();
        serde_json::from_reader(stdin.lock()).map_err(|err| anyhow!(err))?
    };

    // Get Anthropic Base URL from environment variable
    let anthropic_base_url = {
        use std::env::{self, VarError};

        match env::var("ANTHROPIC_BASE_URL") {
            Ok(url) => Some(url),
            Err(VarError::NotPresent) => None,
            Err(err) => {
                error!("ANTHROPIC_BASE_URL read error: {err}");
                None
            },
        }
    };

    let model: Model;
    let ctx_total: usize;
    let ctx_used = claude_data.context_window.tokens_used();

    if let Some(base_url) = anthropic_base_url {
        // If Anthropic Base URL is set, that likely meas we are using custom LLM server.
        // Assume it is LM Studio.
        // Make a request to the LLM API to get a model description.
        match get_lmstudio_model(&base_url) {
            Ok(m) => {
                model = Model::LMStudio(m.id);
                ctx_total = m.loaded_context_length;
            },
            Err(err) => {
                error!("LM Studio API error: {err}");
                model = Model::LMStudio(claude_data.model.display_name);
                ctx_total = claude_data.context_window.context_window_size;
            }
        }
    } else {
        model = Model::Claude(claude_data.model.display_name);
        ctx_total = claude_data.context_window.context_window_size;
    }

    Ok(StatusData {
        model,
        ctx_total,
        ctx_used,
    })
}

#[derive(Deserialize, Debug)]
struct LMStudioData {
    data: Vec<LMStudioDataModel>
}

#[derive(Deserialize, Debug, Clone)]
struct LMStudioDataModel {
    id: String,
    loaded_context_length: usize
}

/// Gets current model information from LM Studio API.
fn get_lmstudio_model(base_url: &str) -> Result<LMStudioDataModel> {
    let data: LMStudioData = ureq::get(format!("{base_url}/api/v0/models"))
        .call()?
        .body_mut()
        .read_json()?;

    if data.data.is_empty() {
        bail!("No models found in response");
    }

    // Assume, the first model is the one we are using.
    // TODO: Find a way to identify the correct model if there are multiple models loaded.
    Ok(data.data[0].clone())
}
