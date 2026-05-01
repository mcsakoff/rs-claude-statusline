use colorz::{Colorize, css};
use std::io::{Result, Write};

use super::{Model, StatusData};

/// Widgets must implement this trait.
pub trait Renderable {
    /// Render the widget to the buffer.
    ///
    /// # Errors
    /// Returns `std::io::Error` on failure.
    fn render(&self, data: &StatusData, buffer: &mut dyn Write) -> Result<()>;
}

impl Renderable for String {
    fn render(&self, _data: &StatusData, buffer: &mut dyn Write) -> Result<()> {
        buffer.write_all(self.as_bytes())
    }
}

/// A main status line object.
#[must_use]
pub struct StatusLine {
    widgets: Vec<Box<dyn Renderable>>,
}

impl StatusLine {
    /// Creates a new status line.
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }

    /// Adds a widget to the status line.
    /// Widget is automatically prepended with a space.
    pub fn add_widget<W>(&mut self, widget: W)
    where
        W: Renderable + 'static,
    {
        if !self.widgets.is_empty() {
            self.widgets.push(Box::new("  ".to_string()));
        }
        self.widgets.push(Box::new(widget));
    }

    /// Adds a widget to the status line without prepending a space.
    pub fn add_widget_no_space<W>(&mut self, widget: W)
    where
        W: Renderable + 'static,
    {
        self.widgets.push(Box::new(widget));
    }
}

/// Default status line with all features enabled.
impl Default for StatusLine {
    fn default() -> Self {
        let mut status_line = StatusLine::new();
        status_line.add_widget(ModelName::new());
        status_line.add_widget(
            ContextBar::new(50)
                .with_percentage()
                .with_usage()
                .with_thresholds(70, 90),
        );
        status_line
    }
}

impl Renderable for StatusLine {
    /// Renders the status line widgets.
    fn render(&self, data: &StatusData, buffer: &mut dyn Write) -> Result<()> {
        for widget in &self.widgets {
            widget.render(data, buffer)?;
        }
        Ok(())
    }
}

/// A widget that displays the name of the model.
pub struct ModelName {}

impl ModelName {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ModelName {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for ModelName {
    fn render(&self, data: &StatusData, buffer: &mut dyn Write) -> Result<()> {
        match &data.model {
            Model::Claude(model_name) => {
                write!(buffer, "{}", model_name.fg(css::Salmon).bold())?;
            }
            Model::LMStudio(model_name) => {
                write!(
                    buffer,
                    "{} [{}]",
                    "LM Studio".white().bold(),
                    model_name.cyan()
                )?;
            }
        }
        Ok(())
    }
}

/// Context bar threshold values in percents.
struct ContextBarThreshold {
    warn_pct: usize,
    crit_pct: usize,
}
/// A widget that displays the context usage statistics.
#[must_use]
pub struct ContextBar {
    width: usize,
    with_percentage: bool,
    with_usage: bool,
    with_thresholds: Option<ContextBarThreshold>,
}

impl ContextBar {
    pub fn new(width: usize) -> Self {
        Self {
            width: width.max(1),
            with_percentage: false,
            with_usage: false,
            with_thresholds: None,
        }
    }

    /// Adds context use percentage to the context bar.
    pub fn with_percentage(mut self) -> Self {
        self.with_percentage = true;
        self
    }

    /// Adds context usage statistics to the context bar.
    pub fn with_usage(mut self) -> Self {
        self.with_usage = true;
        self
    }

    /// Adds context usage thresholds to the context bar.
    ///
    /// Without thresholds, the context bar will have no colors.
    /// With thresholds, the context bar will have green, yellow and red sections.
    ///
    ///  `warn_pct` and `crit_pct` must be between 0 and 100.
    ///  `warn_pct` < `crit_pct`
    ///
    pub fn with_thresholds(mut self, warn_pct: usize, crit_pct: usize) -> Self {
        let warn_pct = warn_pct.clamp(0, 100);
        let crit_pct = crit_pct.clamp(warn_pct, 100);
        self.with_thresholds = Some(ContextBarThreshold { warn_pct, crit_pct });
        self
    }
    /// Converts a percentage to a width in characters.
    ///
    /// `percentage` must be between 0.0 and 100.0.
    ///
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    fn prc_to_width(&self, percentage: f32) -> usize {
        let k = (percentage / 100.0).clamp(0.0, 1.0);
        (k * self.width as f32) as usize
    }
}

const FILL_CHAR: &str = "▓";
const EMPTY_CHAR: &str = "░";

impl Renderable for ContextBar {
    #[allow(clippy::cast_precision_loss)]
    fn render(&self, data: &StatusData, buffer: &mut dyn Write) -> Result<()> {
        let percents = data.ctx_usage_pct();
        let filled = self.prc_to_width(percents);

        if let Some(trh) = &self.with_thresholds {
            // Four sections: green, yellow, red and empty
            let boundary_green = self.prc_to_width(trh.warn_pct as f32);
            let boundary_yellow = self.prc_to_width(trh.crit_pct as f32);

            let green: usize;
            let yellow: usize;
            let red: usize;
            if filled <= boundary_green {
                green = filled;
                yellow = 0;
                red = 0;
            } else if filled <= boundary_yellow {
                green = boundary_green;
                yellow = filled - boundary_green;
                red = 0;
            } else {
                green = boundary_green;
                yellow = boundary_yellow - boundary_green;
                red = filled - boundary_yellow;
            }
            let empty = self.width - (green + yellow + red);

            if green > 0 {
                write!(buffer, "{}", FILL_CHAR.repeat(green).green())?;
            }
            if yellow > 0 {
                write!(buffer, "{}", FILL_CHAR.repeat(yellow).yellow())?;
            }
            if red > 0 {
                write!(buffer, "{}", FILL_CHAR.repeat(red).red())?;
            }
            write!(buffer, "{}", EMPTY_CHAR.repeat(empty))?;
        } else {
            // Two sections: filled and empty
            let empty = self.width - filled;
            if filled > 0 {
                write!(buffer, "{}", FILL_CHAR.repeat(filled))?;
            }
            write!(buffer, "{}", EMPTY_CHAR.repeat(empty))?;
        }

        if self.with_percentage {
            let mut pct: String = format!("{percents:0.1}%");
            if let Some(trh) = &self.with_thresholds {
                if percents > trh.crit_pct as f32 {
                    pct = pct.red().to_string();
                } else if percents > trh.warn_pct as f32 {
                    pct = pct.yellow().to_string();
                } else {
                    pct = pct.green().to_string();
                }
            }
            write!(buffer, " {pct}")?;
        }

        if self.with_usage {
            let used = (data.ctx_used as f32) / 1000.0;
            let total = (data.ctx_total as f32) / 1000.0;
            write!(buffer, " ({used:0.1}k/{total:0.1}k)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Model;

    #[test]
    fn string_as_a_widget() {
        let s = "!!! [test]";
        let w = s.to_string();
        let data = StatusData {
            model: Model::LMStudio(String::new()),
            ctx_used: 0,
            ctx_total: 0,
        };
        let mut output = vec![];
        w.render(&data, &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), s);
    }

    #[test]
    fn model_name_widget() {
        struct TCase {
            model: Model,
            output: &'static str,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                model: Model::Claude("Opus 4.7".to_string()),
                output: "\u{1b}[1m\u{1b}[38;2;250;128;114mOpus 4.7\u{1b}[22m\u{1b}[39m",
            },
            TCase {
                model: Model::LMStudio("qwen3.6-35b-a3b-opus-4.7-dist".to_string()),
                output: "\u{1b}[1m\u{1b}[37mLM Studio\u{1b}[22m\u{1b}[39m [\u{1b}[36mqwen3.6-35b-a3b-opus-4.7-dist\u{1b}[39m]",
            },
        ];
        let model_name = ModelName::default();
        for (ti, tc) in tests.iter().enumerate() {
            let data = StatusData {
                model: tc.model.clone(),
                ctx_used: 0,
                ctx_total: 0,
            };
            let mut output = vec![];
            model_name.render(&data, &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), tc.output, "case #{ti}");
        }
    }

    #[test]
    fn context_bar_widget() {
        struct TCase {
            used: usize,
            total: usize,
            bar: &'static str,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                used: 0,
                total: 100_000,
                bar: "░░░░░░░░░░░░░░░░░░░░ 0.0% (0.0k/100.0k)",
            },
            TCase {
                used: 20_000,
                total: 100_000,
                bar: "▓▓▓▓░░░░░░░░░░░░░░░░ 20.0% (20.0k/100.0k)",
            },
            TCase {
                used: 50_000,
                total: 100_000,
                bar: "▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░ 50.0% (50.0k/100.0k)",
            },
            TCase {
                used: 75_000,
                total: 100_000,
                bar: "▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░ 75.0% (75.0k/100.0k)",
            },
            TCase {
                used: 100_000,
                total: 100_000,
                bar: "▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 100.0% (100.0k/100.0k)",
            },
            TCase {
                used: 150_000,
                total: 100_000,
                bar: "▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 150.0% (150.0k/100.0k)",
            },
        ];

        let bar = ContextBar::new(20).with_percentage().with_usage();
        for (ti, tc) in tests.iter().enumerate() {
            let data = StatusData {
                model: Model::Claude("".into()),
                ctx_used: tc.used,
                ctx_total: tc.total,
            };
            let mut output = vec![];
            bar.render(&data, &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), tc.bar, "case #{ti}");
        }
    }

    #[test]
    fn context_bar_widget_color() {
        struct TCase {
            used: usize,
            total: usize,
            bar: &'static str,
        }
        let tests: Vec<TCase> = vec![
            TCase {
                used: 0,
                total: 100_000,
                bar: "░░░░░░░░░░░░░░░░░░░░ \u{1b}[32m0.0%\u{1b}[39m",
            },
            TCase {
                used: 50_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓\u{1b}[39m░░░░░░░░░░ \u{1b}[32m50.0%\u{1b}[39m",
            },
            TCase {
                used: 70_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m░░░░░░ \u{1b}[32m70.0%\u{1b}[39m",
            },
            TCase {
                used: 80_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m\u{1b}[33m▓▓\u{1b}[39m░░░░ \u{1b}[33m80.0%\u{1b}[39m",
            },
            TCase {
                used: 90_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m\u{1b}[33m▓▓▓▓\u{1b}[39m░░ \u{1b}[33m90.0%\u{1b}[39m",
            },
            TCase {
                used: 95_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m\u{1b}[33m▓▓▓▓\u{1b}[39m\u{1b}[31m▓\u{1b}[39m░ \u{1b}[31m95.0%\u{1b}[39m",
            },
            TCase {
                used: 100_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m\u{1b}[33m▓▓▓▓\u{1b}[39m\u{1b}[31m▓▓\u{1b}[39m \u{1b}[31m100.0%\u{1b}[39m",
            },
            TCase {
                used: 150_000,
                total: 100_000,
                bar: "\u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m\u{1b}[33m▓▓▓▓\u{1b}[39m\u{1b}[31m▓▓\u{1b}[39m \u{1b}[31m150.0%\u{1b}[39m",
            },
        ];

        let bar = ContextBar::new(20)
            .with_percentage()
            .with_thresholds(70, 90);
        for (ti, tc) in tests.iter().enumerate() {
            let data = StatusData {
                model: Model::Claude("".into()),
                ctx_used: tc.used,
                ctx_total: tc.total,
            };
            let mut output = vec![];
            bar.render(&data, &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), tc.bar, "case #{ti}");
        }
    }

    #[test]
    fn status_line() {
        let mut status_line = StatusLine::default();
        status_line.add_widget_no_space("!!!".to_string());
        let data = StatusData {
            model: Model::LMStudio("test".into()),
            ctx_used: 32_000,
            ctx_total: 64_000,
        };
        let mut output = vec![];
        status_line.render(&data, &mut output).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\u{1b}[1m\u{1b}[37mLM Studio\u{1b}[22m\u{1b}[39m [\u{1b}[36mtest\u{1b}[39m]  \u{1b}[32m▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\u{1b}[39m░░░░░░░░░░░░░░░░░░░░░░░░░ \u{1b}[32m50.0%\u{1b}[39m (32.0k/64.0k)!!!"
        );
    }
}
