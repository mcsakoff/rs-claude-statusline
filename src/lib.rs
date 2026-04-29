#![allow(clippy::must_use_candidate)]

use anyhow::{Result, anyhow, bail};
use log::error;
use serde::Deserialize;

pub mod widgets;

#[derive(Clone)]
pub enum Model {
    Claude(String),
    LMStudio(String),
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::Claude(name) => write!(f, "Claude({name})"),
            Model::LMStudio(name) => write!(f, "LMStudio({name})"),
        }
    }
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
    #[allow(clippy::cast_precision_loss)]
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

#[derive(Deserialize, Debug, Clone)]
#[allow(clippy::struct_field_names)]
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
/// - a reader (typically stdin, passed by Claude Code)
/// - environment variables
/// - LLM API (optional)
///
/// # Errors
/// - Returns an error if reading from the reader fails
/// - Returns an error if reading environment variables fails
/// - Returns an error if reading LLM API fails
///
pub fn collect_data<R: std::io::Read>(reader: R) -> Result<StatusData> {
    // Read clade code data from reader
    let claude_data: ClaudeData = {
        serde_json::from_reader(reader).map_err(|err| anyhow!(err))?
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
            }
        }
    };

    let model: Model;
    let ctx_total: usize;
    let ctx_used = claude_data.context_window.tokens_used();

    if let Some(base_url) = anthropic_base_url {
        // If Anthropic Base URL is set, that likely means we are using custom LLM server.
        // Assume it is LM Studio.
        // Make a request to the LLM API to get a model description.
        match get_lmstudio_model(&base_url) {
            Ok(m) => {
                model = Model::LMStudio(m.id);
                ctx_total = m.loaded_context_length;
            }
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
    data: Vec<LMStudioDataModel>,
}

#[derive(Deserialize, Debug)]
struct LMStudioDataModel {
    id: String,
    loaded_context_length: usize,
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
    Ok(data.data.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_usage_pct() {
        struct TCase {
            ctx_total: usize,
            ctx_used: usize,
            expected: f32,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                ctx_total: 1000,
                ctx_used: 250,
                expected: 25.0,
            },
            TCase {
                ctx_total: 0,
                ctx_used: 0,
                expected: 0.0,
            },
            TCase {
                ctx_total: 500,
                ctx_used: 500,
                expected: 100.0,
            },
            TCase {
                ctx_total: 100,
                ctx_used: 150,
                expected: 150.0,
            },
            TCase {
                ctx_total: 1000,
                ctx_used: 333,
                expected: 33.3,
            },
        ];

        for (ti, tc) in tests.iter().enumerate() {
            let data = StatusData {
                model: Model::Claude("test".to_string()),
                ctx_total: tc.ctx_total,
                ctx_used: tc.ctx_used,
            };
            let result = data.ctx_usage_pct();
            assert!((result - tc.expected).abs() < 0.1, "case #{ti}");
        }
    }

    #[test]
    fn claude_ctx_usage_input_total() {
        struct TCase {
            input_tokens: usize,
            cache_creation_input_tokens: usize,
            cache_read_input_tokens: usize,
            expected: usize,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                input_tokens: 0,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
                expected: 0,
            },
            TCase {
                input_tokens: 1000,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
                expected: 1000,
            },
            TCase {
                input_tokens: 1000,
                cache_creation_input_tokens: 2000,
                cache_read_input_tokens: 3000,
                expected: 6000,
            },
        ];

        for (ti, tc) in tests.iter().enumerate() {
            let usage = ClaudeDataCtxUsage {
                input_tokens: tc.input_tokens,
                cache_creation_input_tokens: tc.cache_creation_input_tokens,
                cache_read_input_tokens: tc.cache_read_input_tokens,
            };
            let result = usage.input_total();
            assert_eq!(result, tc.expected, "case #{ti}");
        }
    }

    #[test]
    fn claude_ctx_tokens_used() {
        struct TCase {
            current_usage: Option<ClaudeDataCtxUsage>,
            expected: usize,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                current_usage: None,
                expected: 0,
            },
            TCase {
                current_usage: Some(ClaudeDataCtxUsage {
                    input_tokens: 1000,
                    cache_creation_input_tokens: 2000,
                    cache_read_input_tokens: 3000,
                }),
                expected: 6000,
            },
        ];

        for (ti, tc) in tests.iter().enumerate() {
            let ctx = ClaudeDataCtx {
                context_window_size: 100_000,
                current_usage: tc.current_usage.clone(),
            };
            let result = ctx.tokens_used();
            assert_eq!(result, tc.expected, "case #{ti}");
        }
    }

    #[test]
    fn get_lmstudio_model_tests() {
        use httpmock::prelude::*;

        struct TCase {
            json: Option<&'static str>,
            expected: TestResult,
        }
        enum TestResult {
            Model { id: String, ctx: usize },
            Error(&'static str),
        }

        let tests: Vec<TCase> = vec![
            TCase {
                json: Some(
                    r#"{"data":[{"id":"Llama-3.1-8B-Instruct","loaded_context_length":4096}]}"#,
                ),
                expected: TestResult::Model {
                    id: "Llama-3.1-8B-Instruct".into(),
                    ctx: 4096,
                },
            },
            TCase {
                json: Some(r#"{"data":[]}"#),
                expected: TestResult::Error("No models found"),
            },
            TCase {
                json: None,
                expected: TestResult::Error("Connection refused"),
            },
        ];

        for (ti, tc) in tests.iter().enumerate() {
            let result = match tc.json {
                Some(json) => {
                    let server = MockServer::start();
                    let mock = server.mock(|when, then| {
                        when.path("/api/v0/models");
                        then.status(200)
                            .header("Content-Type", "application/json")
                            .body(json.as_bytes());
                    });
                    let result = super::get_lmstudio_model(&server.url(""));
                    mock.assert();
                    result
                }
                None => super::get_lmstudio_model("http://127.0.0.1:1"),
            };

            match (&tc.expected, &result) {
                (TestResult::Model { id, ctx }, Ok(model)) => {
                    assert_eq!(model.id, *id, "case #{ti}");
                    assert_eq!(model.loaded_context_length, *ctx, "case #{ti}");
                }
                (TestResult::Error(expected), Err(err)) => {
                    assert!(err.to_string().contains(expected), "case #{ti}");
                }
                _ => panic!("case #{ti}: mismatch"),
            }
        }
    }

    #[test]
    fn collect_data_tests() {
        use std::env;
        use std::io::Cursor;

        struct TData {
            anthropic_base_url_set: bool,
            stdin_json: &'static str,
            lmstudio_json: Option<&'static str>,
        }

        struct TExpected {
            model: &'static str,
            ctx_total: usize,
            ctx_used: usize,
        }

        struct TCase {
            name: &'static str,
            data: TData,
            expected: TExpected,
        }

        let tests: Vec<TCase> = vec![
            // Case 0: Claude with token usage
            TCase {
                name: "Claude with usage",
                data: TData {
                    anthropic_base_url_set: false,
                    stdin_json: include_str!("../tests/claude1.json"),
                    lmstudio_json: None,
                },
                expected: TExpected {
                    model: "Claude(qwen3.6-35b-a3b)",
                    ctx_total: 200_000,
                    ctx_used: 28584 + 0 + 26946,
                },
            },
            // Case 1: Claude without usage (claude2.json has current_usage: null)
            TCase {
                name: "Claude without usage",
                data: TData {
                    anthropic_base_url_set: false,
                    stdin_json: include_str!("../tests/claude2.json"),
                    lmstudio_json: None,
                },
                expected: TExpected {
                    model: "Claude(qwen3.6-35b-a3b)",
                    ctx_total: 200_000,
                    ctx_used: 0,
                },
            },
            // Case 2: LM Studio path with failed API (falls back to Claude defaults)
            TCase {
                name: "LM Studio fallback",
                data: TData {
                    anthropic_base_url_set: true,
                    stdin_json: include_str!("../tests/claude1.json"),
                    lmstudio_json: None,
                },
                expected: TExpected {
                    model: "LMStudio(qwen3.6-35b-a3b)",
                    ctx_total: 200_000,
                    ctx_used: 28584 + 0 + 26946,
                },
            },
            // Case 3: LM Studio success
            TCase {
                name: "LM Studio success",
                data: TData {
                    anthropic_base_url_set: true,
                    stdin_json: include_str!("../tests/claude1.json"),
                    lmstudio_json: Some(include_str!("../tests/lmstudio.modes.json")),
                },
                expected: TExpected {
                    model: "LMStudio(qwen3.6-35b-a3b-opus-4.6)",
                    ctx_total: 200_000,
                    ctx_used: 28584 + 0 + 26946,
                },
            },
        ];

        // Save original env var state and restore it before each test case
        let original_anthropic = env::var("ANTHROPIC_BASE_URL").ok();

        for tc in tests {
            let result = if tc.data.anthropic_base_url_set {
                let server = httpmock::MockServer::start();
                if let Some(json) = tc.data.lmstudio_json {
                    unsafe { env::set_var("ANTHROPIC_BASE_URL", &server.url("")) };
                    server.mock(|when, then| {
                        when.path("/api/v0/models");
                        then.status(200)
                            .header("Content-Type", "application/json")
                            .body(json.as_bytes());
                    });
                } else {
                    unsafe { env::set_var("ANTHROPIC_BASE_URL", "http://127.0.0.1:1") };
                }
                collect_data(Cursor::new(tc.data.stdin_json.as_bytes()))
            } else {
                unsafe { env::remove_var("ANTHROPIC_BASE_URL") };
                collect_data(Cursor::new(tc.data.stdin_json.as_bytes()))
            };

            // Restore original env var state
            match &original_anthropic {
                Some(val) => unsafe { env::set_var("ANTHROPIC_BASE_URL", val) },
                None => unsafe { env::remove_var("ANTHROPIC_BASE_URL") },
            }

            match result {
                Ok(data) => {
                    assert_eq!(
                        data.model.to_string(),
                        tc.expected.model,
                        "case: {}",
                        tc.name
                    );
                    assert_eq!(data.ctx_total, tc.expected.ctx_total, "case: {}", tc.name);
                    assert_eq!(data.ctx_used, tc.expected.ctx_used, "case: {}", tc.name);
                }
                Err(err) => panic!("case: {name}; unexpected error: {err}", name = tc.name),
            }
        }
    }
}
