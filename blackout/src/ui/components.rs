use ratatui::layout::Position;

use crate::ui::prelude::*;

pub fn render_initial_check(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::bordered().title("Checking vault status..."), area);
}

pub fn get_helper_text(state: &AppState) -> Line<'static> {
    let text = match state {
        AppState::InitialCheck | AppState::SnapshotList => "(Esc) Quit",
        AppState::UnlockPrompt(_) => "(↵) Submit | (Esc) Quit",
        AppState::VaultLocked => "(Esc) Quit | (any) Unlock vault",
        AppState::EntriesList => {
            "(Esc) Quit | (e) Edit | (↵) Select | (⌫) Delete | (n) New | (x) Lock | (?) Options "
        }
        AppState::NewEntryForm(_)
        | AppState::UpdateEntry(_)
        | AppState::ChangeMasterPassword(_) => {
            "(Esc) Back | (Tab) Next field | (BackTab) Prev field | (↵) Submiti | (F2) Toggle password visibility"
        }
        AppState::ViewEntry(..) => {
            "(Esc) Back | (e) Edit | (⌫) Delete | (↵) Copy password | (F2) Toggle password visibility"
        }
        AppState::ConfirmAction { .. } => "(Esc) Cancel | (↵) Confirm",
        AppState::Settings(_) => "(Esc) Back | (↑ and ↓) Navigate | (↵) Select",
    };
    Line::from(text).dim()
}

pub fn get_status_text(app: &App) -> Line<'_> {
    match &app.status_message {
        Some(msg) => Line::from(msg.as_str()).dim(),
        None => Line::from(format!("vault v{}", app.vault_version)).dim(),
    }
}

pub fn render_locked_vault(frame: &mut Frame, area: Rect) {
    let [area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&Layout::vertical([Constraint::Percentage(100)]));
    frame.render_widget(Line::from("Your vault is locked!").bold().centered(), area);
}

pub fn render_pending_action(frame: &mut Frame, area: Rect, action: &PendingAction) {
    let prompt = action.get_prompt_text();
    let [confirm_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&Layout::vertical([Constraint::Percentage(50)]));
    let text = vec![
        Line::from(prompt.to_string()).centered(),
        Line::from(""),
        Line::from(" [Y/Enter]  |  [N/Esc] ").centered().bold(),
    ];
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), confirm_area);
}

pub fn render_form(f: &mut Frame, area: Rect, title: &str, fields: &[FieldConfig], app: &App) {
    let horizontal = Layout::horizontal([Constraint::Fill(1)]);
    let [form_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(50))
        .layout(&horizontal);

    let mut lines = Vec::new();

    lines.push(Line::from(Span::styled(
        title.to_uppercase().to_string(),
        Style::default().bold(),
    )));
    lines.push(Line::from(""));

    for (i, field) in fields.iter().enumerate() {
        let is_active = i == app.form_state.current_field;
        let mut content = app.get_input_for_field(i).to_string();

        if field.is_password && app.form_state.obscure_inputs && !field.show_password {
            content = "*".repeat(content.len());
        }

        let label_formatted = format!("{}: ", field.label);

        if is_active {
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", field.label), Style::default().bold()),
                Span::raw(content.clone()),
            ]));
            let cursor_x = form_area.x
                + label_formatted.chars().count() as u16
                + app.form_state.cursor_index as u16;
            let cursor_y = form_area.y + 2 + (i as u16 * 2);

            f.set_cursor_position(Position::new(cursor_x, cursor_y));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", field.label), Style::default()),
                Span::styled(content, Style::default()),
            ]));
        }

        lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(lines), form_area);
}

pub fn get_title_text(app: &App) -> Line<'static> {
    let version = env!("CARGO_PKG_VERSION");
    let mut title_text = Line::from(Span::from(format!("Blackout - v{version}")));

    if app.dev_mode {
        title_text.push_span(Span::raw(" - DEV MODE"))
    }

    title_text
}
