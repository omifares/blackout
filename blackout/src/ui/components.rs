use ratatui::layout::Position;

use crate::state::FormState;
use crate::ui::prelude::*;

pub fn render_initial_check(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::bordered().title("Checking vault status..."), area);
}

pub fn get_helper_text(state: &AppState) -> Line<'static> {
    let text = match state {
        AppState::InitialCheck => "(Esc) Quit",
        AppState::SnapshotList => "(Esc) Back | (↑ and ↓) Navigate | (↵) Select",
        AppState::UnlockPrompt(_) => "(↵) Submit | (Esc) Quit",
        AppState::VaultLocked => "(Esc) Quit | (F3) Gen Pass | (any) Unlock vault",
        AppState::EntriesList => {
            "(Esc) Quit | (e) Edit | (↵) Select | (⌫) Delete | (n) New | (x) Lock | (?) Options "
        }
        AppState::NewEntryForm(_)
        | AppState::UpdateEntry(_)
        | AppState::ChangeMasterPassword(_) => {
            "(Esc) Back | (Tab) Next field | (BackTab) Prev field | (↵) Submit | (F2) Toggle password visibility"
        }
        AppState::ViewEntry(..) => {
            "(Esc) Back | (e) Edit | (⌫) Delete | (↵) Copy password | (F2) Toggle password visibility"
        }
        AppState::ConfirmAction { .. } => "(Esc) Cancel | (↵) Confirm",
        AppState::Settings(_) => "(Esc) Back | (↑ and ↓) Navigate | (↵) Select",
        AppState::PasswordGenerator(_) => "(Esc) Back | (↵) Copy | (r) Regen",
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

pub fn render_password_generator_form(
    f: &mut Frame,
    area: Rect,
    title: &str,
    fields: &[FieldConfig],
    form_state: &FormState,
    generated_password: Option<&String>,
) {
    let horizontal = Layout::horizontal([Constraint::Fill(1)]);
    let [form_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(50))
        .layout(&horizontal);

    let vertical_chunks = if generated_password.is_some() {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
    } else {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
    };
    let [title_area, password_area, fields_area] = form_area.layout(&vertical_chunks);

    // Render title
    f.render_widget(Paragraph::new(title).bold().centered(), title_area);

    // Render generated password
    let mut pwd_line = Vec::new();
    if let Some(password) = generated_password {
        pwd_line.push(Line::from(Span::styled(
            "Generated Password: ".to_string() + password.as_str(),
            Style::default(),
        )));
    } else {
        pwd_line.push(Line::from(Span::styled(
            "Press 'r' to generate a password",
            Style::default(),
        )));
    }

    f.render_widget(Paragraph::new(pwd_line).centered(), password_area);

    // Split fields area into two columns
    let horizontal_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
    let [left_area, right_area] = fields_area.layout(&horizontal_chunks);

    // Determine split index
    let left_len = fields.len() / 2;
    let left_fields = &fields[0..left_len];
    let right_fields = &fields[left_len..];

    // Helper to render column lines (without cursor)
    let render_column_lines =
        |_area: Rect, fields_slice: &[FieldConfig], offset: usize| -> Vec<Line<'static>> {
            let mut lines = Vec::new();
            for (i, field) in fields_slice.iter().enumerate() {
                let global_idx = offset + i;
                let content = form_state
                    .fields
                    .get(global_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                let display_content =
                    if field.is_password && form_state.obscure_inputs && !field.show_password {
                        "*".repeat(content.len())
                    } else {
                        content.clone()
                    };

                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", field.label), Style::default()),
                    Span::styled(display_content, Style::default()),
                ]));

                lines.push(Line::from(""));
            }
            lines
        };

    // Render left column
    let left_lines = render_column_lines(left_area, left_fields, 0);
    f.render_widget(Paragraph::new(left_lines), left_area);

    // Render right column
    let right_lines = render_column_lines(right_area, right_fields, left_len);
    f.render_widget(Paragraph::new(right_lines), right_area);

    // Set cursor if there is an active field
    if form_state.current_field < fields.len() {
        let active_in_left = form_state.current_field < left_len;
        let local_idx = if active_in_left {
            form_state.current_field
        } else {
            form_state.current_field - left_len
        };
        let area = if active_in_left {
            left_area
        } else {
            right_area
        };
        let field = &fields[form_state.current_field];
        let label_formatted = format!("{}: ", field.label);
        let cursor_x =
            area.x + label_formatted.chars().count() as u16 + form_state.cursor_index as u16;
        let cursor_y = area.y + 2 + (local_idx as u16 * 2);
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

pub fn get_title_text(app: &App) -> Line<'static> {
    let version = env!("CARGO_PKG_VERSION");
    let mut title_text = Line::from(Span::from(format!("Blackout - v{version}")));

    if app.dev_mode {
        title_text.push_span(Span::raw(" - DEV MODE"))
    }

    title_text
}
