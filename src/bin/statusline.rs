use anyhow::Result;
use log::error;
use std::process::ExitCode;

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
    use claude_statusline::widgets::Renderable;

    // collect required data from various sources
    let status_data = claude_statusline::collect_data(std::io::stdin().lock())?;

    // build the status line
    let status_line = claude_statusline::widgets::StatusLine::default();

    // render the status line
    let mut stdout = std::io::stdout().lock();
    status_line.render(&status_data, &mut stdout)?;
    Ok(())
}
