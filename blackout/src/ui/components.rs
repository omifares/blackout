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
        AppState::UnlockPrompt => "(↵) Submit | (Esc) Quit",
        AppState::VaultLocked => "(Esc) Quit | (F3) Gen Pass | (any) Unlock vault",
        AppState::EntriesList => {
            "(Esc) Quit | (e) Edit | (↵) Select | (⌫) Delete | (n) New | (x) Lock | (?) Options "
        }
        AppState::NewEntryForm | AppState::UpdateEntry | AppState::ChangeMasterPassword => {
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

pub fn render_form(f: &mut Frame, area: Rect, title: &str, app: &App) {
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

    for (i, field) in app.form_state.fields.iter().enumerate() {
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
    generated_password: Option<&String>,
    app: &App,
) {
    let layout = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Fill(1),
    ]);
    let chunks = layout.split(area);
    let [title_area, password_area, fields_area] = chunks.into_iter().collect::<Vec<_>>()[..]
        .try_into()
        .unwrap();

    render_title(f, *title_area, title);
    render_generated_password(f, *password_area, generated_password);
    render_dynamic_fields(f, *fields_area, &app.form_state.fields, &app.form_state);
}

fn render_generated_password(f: &mut Frame, area: Rect, password: Option<&String>) {
    let text = match password {
        Some(p) => format!("Generated Password: {}", p),
        None => "Press 'r' to generate a password".to_string(),
    };
    f.render_widget(Paragraph::new(text).centered(), area);
}

fn render_dynamic_fields(f: &mut Frame, area: Rect, fields: &[FieldConfig], state: &FormState) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .horizontal_margin(20)
        .flex(ratatui::layout::Flex::Center);

    let chunks = layout.split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    let mid = (fields.len() + 1) / 2;
    let (left, right) = fields.split_at(mid);

    render_column(f, left_area, left, state, 0);
    render_column(f, right_area, right, state, mid);
}

fn render_column(
    f: &mut Frame,
    area: Rect,
    fields: &[FieldConfig],
    state: &FormState,
    offset: usize,
) {
    let constraints = std::iter::repeat(Constraint::Length(2)).take(fields.len());
    let chunks = Layout::vertical(constraints).split(area);

    for (i, field) in fields.iter().enumerate() {
        let field_idx = i + offset;
        let is_selected = field_idx == state.current_field;

        let style = if is_selected {
            Style::default().bold().underlined()
        } else {
            Style::default()
        };

        let content = match &field.value {
            FieldValue::Text(s) => s.clone(),
            FieldValue::Number(n) => n.to_string(),
            FieldValue::Choice(idx) => {
                if let FieldType::Choice(menu) = &field.field_type {
                    format!("< {} >", menu.options().get(*idx).unwrap_or(&""))
                } else {
                    "".to_string()
                }
            }
            FieldValue::Boolean(b) => {
                if *b {
                    "[X]".to_string()
                } else {
                    "[ ]".to_string()
                }
            }
        };

        let line = Line::from(vec![
            Span::styled(format!("{}: ", field.label), Style::default()),
            Span::styled(content, style),
        ]);

        f.render_widget(Paragraph::new(line), chunks[i]);

        if is_selected {
            if let FieldValue::Text(_) = &field.value {
                let x = chunks[i].x + (field.label.len() as u16) + 2 + state.cursor_index as u16;
                let y = chunks[i].y;
                f.set_cursor_position(Position::new(x, y));
            }
        }
    }
}

fn render_title(f: &mut Frame, area: Rect, title: &str) {
    f.render_widget(Paragraph::new(title).bold().centered(), area);
}

pub fn get_title_text(app: &App) -> Line<'static> {
    let version = env!("CARGO_PKG_VERSION");
    let mut title_text = Line::from(Span::from(format!("Blackout - v{version}")));

    if app.dev_mode {
        title_text.push_span(Span::raw(" - DEV MODE"))
    }

    title_text
}
