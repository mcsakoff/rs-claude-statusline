use anyhow::{anyhow, Result};
use serde::Deserialize;

pub mod widgets;

/// A status data object.
///
/// Contains information required by status widgets.
///
pub struct StatusData {
    pub model: String,
    pub ctx_total: usize,
    pub ctx_used: usize,
}

impl StatusData {
    /// Returns the percentage of the context used.
    pub fn ctx_usage_pct(&self) -> f32 {
        (self.ctx_used as f32 / self.ctx_total as f32) * 100.0
    }
}

/// Collects status data from:
/// - stdin (passed by ClaudeCode)
/// - environment variables
/// - LLM API (optional)
pub fn collect_data() -> Result<StatusData> {
    let claude_data: ClaudeData = {
        use serde_json;
        let stdin = std::io::stdin();
        serde_json::from_reader(stdin.lock()).map_err(|err| anyhow!(err))?
    };
    Ok(StatusData {
        model: claude_data.model.display_name,
        ctx_total: claude_data.context_window.context_window_size,
        ctx_used: claude_data.context_window.tokens_used(),
    })
}

#[derive(Deserialize, Debug)]
struct ClaudeData {
    model: ClaudeDataModel,
    context_window: ClaudeDataCtx
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
