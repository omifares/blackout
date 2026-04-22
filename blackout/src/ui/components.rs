use crate::ui::prelude::*;

pub fn render_initial_check(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::bordered().title("Checking vault status..."), area);
}

pub fn get_helper_text(state: &AppState) -> Line<'static> {
    let text = match state {
        AppState::InitialCheck | AppState::SnapshotList => "(Esc) Quit",
        AppState::UnlockPrompt => "(↵) Submit | (Esc) Quit",
        AppState::VaultLocked => "(Esc) Quit | (any) Unlock vault",
        AppState::EntriesList => {
            "(Esc) Quit | (e) Edit | (↵) Select | (⌫) Delete | (n) New | (x) Lock | (?) Options "
        }
        AppState::NewEntryForm(_)
        | AppState::UpdateEntry(_)
        | AppState::ChangeMasterPassword(_) => {
            "(Esc) Back | (Tab) Next field | (BackTab) Prev field | (↵) Submit"
        }
        AppState::ViewEntry(_) => {
            "(Esc) Back | (x) Lock | (e) Edit | (⌫) Delete | (↵) Copy password | (v) Toggle password visibility"
        }
        AppState::ConfirmEntryDelete => "(Esc) Cancel | (↵) Confirm",
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

pub fn render_unlock_prompt(frame: &mut Frame, area: Rect, input: &str, app: &App) {
    let horizontal = Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]);
    let [label_area, pass_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&horizontal);

    let label = Line::from("Enter vault password:").bold();
    let cursor = if app.is_cursor_visible() { "█" } else { " " };
    let mask = "*".repeat(input.chars().count());
    let pass_display = format!("{}{}", mask, cursor);
    let pass_paragraph = Paragraph::new(pass_display);

    frame.render_widget(label, label_area);
    frame.render_widget(pass_paragraph, pass_area);
}

pub fn render_locked_vault(frame: &mut Frame, area: Rect) {
    let [area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&Layout::vertical([Constraint::Percentage(100)]));
    frame.render_widget(Line::from("Your vault is locked!").bold().centered(), area);
}

pub fn render_delete_confirmation(frame: &mut Frame, area: Rect) {
    let [confirm_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&Layout::vertical([Constraint::Percentage(50)]));
    let text = vec![
        Line::from("Are you sure you want to delete this entry?").centered(),
        Line::from(""),
        Line::from(" [y]es  |  [n]o ").centered().bold(),
    ];
    frame.render_widget(Paragraph::new(text), confirm_area);
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

    let cursor_char = if app.is_cursor_visible() { "█" } else { " " };

    for (i, field) in fields.iter().enumerate() {
        let is_active = i == app.form_state.current_index;
        let mut content = app.get_input_for_field(i).to_string();

        if field.is_password && app.form_state.obscure_inputs && !field.show_password {
            content = "*".repeat(content.len());
        }

        if is_active {
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", field.label), Style::default().bold()),
                Span::raw(content),
                Span::styled(cursor_char, Style::default()),
            ]));
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
