pub mod components;
pub mod prelude;
pub mod views;

use components::*;
use prelude::*;

pub fn render(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .spacing(1)
    .horizontal_margin(3)
    .vertical_margin(1);

    let [top, area, status_area, bottom] = frame.area().layout(&vertical);

    frame.render_widget(get_title_text(app).bold().centered(), top);

    match &app.state {
        AppState::InitialCheck => components::render_initial_check(frame, area),

        AppState::UnlockPrompt => components::render_form(frame, area, "Unlock Vault", app),
        AppState::VaultLocked => components::render_locked_vault(frame, area),

        AppState::EntriesList => views::entry_list::render(frame, area, app),
        AppState::ViewEntry(_) => components::render_form(frame, area, "View entry", app),
        AppState::Settings(_) => views::settings::render(frame, area, app),
        AppState::SnapshotList => views::snapshots::render(frame, area, app),
        AppState::ConfirmAction { action, .. } => {
            components::render_pending_action(frame, area, action)
        }
        AppState::NewEntryForm => components::render_form(frame, area, "New entry", app),
        AppState::UpdateEntry => components::render_form(frame, area, "Edit entry", app),
        AppState::ChangeMasterPassword => {
            components::render_form(frame, area, "Change Master Password", app)
        }
        AppState::PasswordGenerator(state) => {
            components::render_password_generator_form(
                frame,
                area,
                "Pass Generator",
                state.generated_password.as_ref(),
                app,
            );
        }
    }

    // Status & Footer
    frame.render_widget(get_status_text(app).centered(), status_area);
    frame.render_widget(get_helper_text(&app.state).centered(), bottom);
}
