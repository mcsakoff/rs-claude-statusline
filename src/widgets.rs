use colorz::{css, Colorize};

use super::{Model, StatusData};

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

impl Renderable for StatusLine {
    /// Renders the status line widgets.
    fn render(&self, data: &StatusData) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        for widget in &self.widgets {
            write!(&mut output, "{}", widget.render(&data)).unwrap();
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

impl Renderable for ModelName {
    fn render(&self, data: &StatusData) -> String {
        match &data.model {
            Model::Claude(model_name) => model_name.fg(css::Salmon).bold().to_string(),
            Model::LMStudio(model_name) => {
                format!("{} [{}]", "LM Studio".bold(), model_name.cyan())
            }
        }
    }
}

/// A widget that displays the context usage statistics.
pub struct ContextBar {
    width: usize,
    with_percentage: bool,
    with_usage: bool,
}

impl ContextBar {
    pub fn new(width: usize) -> Self {
        Self {
            width: width.clamp(1, usize::MAX),
            with_percentage: false,
            with_usage: false,
        }
    }

    pub fn with_percentage(mut self) -> Self {
        self.with_percentage = true;
        self
    }

    pub fn with_usage(mut self) -> Self {
        self.with_usage = true;
        self
    }

    /// Converts a percentage to a width in characters.
    fn prc_to_width(&self, percentage: f32) -> usize {
        let k = (percentage / 100.0).clamp(0.0, 1.0);
        (k * self.width as f32) as usize
    }
}

impl Renderable for ContextBar {
    fn render(&self, data: &StatusData) -> String {
        use std::fmt::Write;

        let estimated_width = {
            let mut n = self.width;
            if self.with_percentage {
                n += " 00.0%".len();
            }
            if self.with_usage {
                n += " (0000.0k/0000.0k)".len();
            }
            n
        };
        let mut output = String::with_capacity(estimated_width);

        let percents = data.ctx_usage_pct();
        let total = self.width;
        let filled = self.prc_to_width(percents);
        let empty = total - filled;

        write!(&mut output, "{}", "▓".repeat(filled)).unwrap();
        write!(&mut output, "{}", "░".repeat(empty)).unwrap();
        if self.with_percentage {
            write!(&mut output, " {:0.1}%", percents).unwrap();
        }
        if self.with_usage {
            let used = (data.ctx_used as f32) / 1000.0;
            let total = (data.ctx_total as f32) / 1000.0;
            write!(&mut output, " ({used:0.1}k/{total:0.1}k)").unwrap();
        }
        output
    }
}
