mod app;
mod events;
mod ui;

use blackout_core::ipc::{Request, Response};
use crossterm::event::{self, Event, KeyCode};

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use color_eyre::Result;
use ratatui::DefaultTerminal;

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = app::App::new();
    app.check_vault_status(); // Check initial status

    let terminal = ratatui::init();
    let result = run(terminal, &mut app);
    ratatui::restore();
    result
}

pub fn send_command(req: Request) -> Result<Response> {
    let mut stream = UnixStream::connect(blackout_core::ipc::get_socket_path())?;

    let req_json = serde_json::to_string(&req)? + "\n";
    stream.write_all(req_json.as_bytes())?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let response: Response = serde_json::from_str(&response_line)?;
    Ok(response)
}

fn run(mut terminal: DefaultTerminal, app: &mut app::App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        // Pulling
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.code == KeyCode::Esc // Handling ESC to exit
                    && matches!(
                        app.state,
                        app::AppState::InitialCheck
                            | app::AppState::UnlockPrompt
                            | app::AppState::EntriesList
                            | app::AppState::VaultLocked
                    )
            {
                break;
            }

            events::handle_event(app, key);
        }

        // Check vault status silently after 30s
        if app.last_interaction.elapsed() < Duration::from_secs(30) {
            app.check_vault_status();
        } else {
            // > 30s auto-lock vault
            app.lock_application();
        }

        if let Some(time) = app.status_time
            && time.elapsed() >= Duration::from_secs(4)
        {
            app.status_message = None;
            app.status_time = None;
        }
    }
    Ok(())
}
