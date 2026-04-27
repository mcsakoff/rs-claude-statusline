use super::{Model, StatusData};
use colorz::{css, Colorize};
use log::debug;

/// Widgets must implement this trait.
pub trait Renderable {
    fn render(&self, data: &StatusData) -> String;
}

impl Renderable for String {
    fn render(&self, _data: &StatusData) -> String {
        self.clone()
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
    pub fn add_widget<W>(mut self, widget: W) -> Self
    where
        W: Renderable + 'static,
    {
        if !self.widgets.is_empty() {
            self.widgets.push(Box::new("  ".to_string()));
        }
        self.widgets.push(Box::new(widget));
        self
    }

    /// Adds a widget to the status line without prepending a space.
    pub fn add_widget_no_space<W>(mut self, widget: W) -> Self
    where
        W: Renderable + 'static,
    {
        self.widgets.push(Box::new(widget));
        self
    }
}

/// Default status line with all features enabled.
impl Default for StatusLine {
    fn default() -> Self {
        StatusLine::new().add_widget(ModelName::new()).add_widget(
            ContextBar::new(50)
                .with_percentage()
                .with_usage()
                .with_thresholds(70, 90),
        )
    }
}

impl Renderable for StatusLine {
    /// Renders the status line widgets.
    fn render(&self, data: &StatusData) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        for widget in &self.widgets {
            write!(&mut output, "{}", widget.render(data)).unwrap();
        }
        output
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
    fn render(&self, data: &StatusData) -> String {
        match &data.model {
            Model::Claude(model_name) => model_name.fg(css::Salmon).bold().to_string(),
            Model::LMStudio(model_name) => {
                format!("{} [{}]", "LM Studio".white().bold(), model_name.cyan())
            }
        }
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
            width: width.clamp(1, usize::MAX),
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
    fn render(&self, data: &StatusData) -> String {
        use std::fmt::Write;

        let estimated_width = {
            let mut n = self.width * 3; // due to Unicode symbols
            if self.with_percentage {
                n += " 000.0%".len();
            }
            if self.with_usage {
                n += " (0000.0k/0000.0k)".len();
            }
            if self.with_thresholds.is_some() {
                n += 40; // for colors
            }
            n
        };
        debug!("estimated contxt bar width: {estimated_width}");
        let mut output = String::with_capacity(estimated_width);

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
                write!(&mut output, "{}", FILL_CHAR.repeat(green).green()).unwrap();
            }
            if yellow > 0 {
                write!(&mut output, "{}", FILL_CHAR.repeat(yellow).yellow()).unwrap();
            }
            if red > 0 {
                write!(&mut output, "{}", FILL_CHAR.repeat(red).red()).unwrap();
            }
            write!(&mut output, "{}", EMPTY_CHAR.repeat(empty)).unwrap();
        } else {
            // Two sections: filled and empty
            let empty = self.width - filled;
            if filled > 0 {
                write!(&mut output, "{}", FILL_CHAR.repeat(filled)).unwrap();
            }
            write!(&mut output, "{}", EMPTY_CHAR.repeat(empty)).unwrap();
        }

        if self.with_percentage {
            let mut pct: String = format!("{percents:0.1}%");
            if let Some(trh) = &self.with_thresholds {
                if percents > trh.crit_pct as f32 {
                    pct = pct.red().to_string();
                } else if percents > trh.warn_pct as f32 {
                    pct = pct.yellow().to_string();
                }
            }
            write!(&mut output, " {pct}").unwrap();
        }

        if self.with_usage {
            let used = (data.ctx_used as f32) / 1000.0;
            let total = (data.ctx_total as f32) / 1000.0;
            write!(&mut output, " ({used:0.1}k/{total:0.1}k)").unwrap();
        }
        debug!("final contxt bar width: {}", output.len());
        output
    }
}
