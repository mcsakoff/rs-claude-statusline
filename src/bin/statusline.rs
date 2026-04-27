use anyhow::Result;
use log::error;
use std::process::ExitCode;
use claude_statusline::widgets::Renderable;

fn main() -> ExitCode {
    env_logger::init_from_env(
        env_logger::Env::default()
            .filter_or("LOG_LEVEL", "info")
            .write_style_or("LOG_STYLE", "always"),
    );
    if let Err(err) = run() {
        error!("{err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    use claude_statusline::{collect_data};
    use claude_statusline::widgets::{StatusLine, ModelName, ContextBar};

    // collect required data from various sources
    let status_data = collect_data()?;

    // build the status line
    let status_line = StatusLine::new()
        .add_widget(ModelName::new())
        .add_widget(ContextBar::new(50)
            .with_percentage()
            .with_usage()
            .with_thresholds(70, 90));

    // render the status line
    let output = status_line.render(&status_data);
    println!("{output}");
    Ok(())
}
