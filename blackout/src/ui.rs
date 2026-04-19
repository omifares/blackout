use ratatui::widgets::{Block, Cell, Paragraph, Row, Table};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
};

use crate::app::{App, AppState, DetailEntryView, EntryView, ListEntryView};

fn is_cursor_visible() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() % 1000 < 500 // interval: 500ms
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let version = env!("CARGO_PKG_VERSION");
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .spacing(1)
    .horizontal_margin(3)
    .vertical_margin(1);
    let [top, main, status_area, bottom] = frame.area().layout(&vertical);

    frame.render_widget(Line::from(Span::from(format!("Blackout - v{version}")).bold()).centered(), top);

    match &app.state {
        AppState::InitialCheck => render_initial_check(frame, main),
        AppState::UnlockPrompt => render_unlock_prompt(frame, main, &app.input_buffer),
        AppState::VaultLocked => render_locked_vault(frame, main),
        AppState::EntriesList => render_entries_list(frame, main, app),
        AppState::NewEntryForm => render_form(frame, main, "New entry", app),
        AppState::UpdateEntry => render_form(frame, main, "Edit entry", app),
        AppState::ViewEntry(view) => render_view_entry(frame, main, app, view),
        AppState::ConfirmEntryDelete => render_delete_confirmation(frame, main),
    }

    // Status & Footer
    frame.render_widget(get_status_text(&app).centered(), status_area);
    frame.render_widget(get_helper_text(&app.state).centered(), bottom);
}

fn get_helper_text(state: &AppState) -> Line<'static> {
    let text = match state {
        AppState::InitialCheck => "(Esc) Quit",
        AppState::UnlockPrompt => "(Enter) Submit | (Esc) Quit",
        AppState::VaultLocked => "(Esc) Quit | (any) Unlock vault",
        AppState::EntriesList => {
            "(Esc) Quit | (e) Edit | (↵) Select | (⌫) Delete | (n) New | (x) Lock"
        }
        AppState::NewEntryForm | AppState::UpdateEntry => {
            "(Tab) Next field | (BackTab) Prev field | (Enter) Submit | (Esc) Cancel"
        }
        AppState::ViewEntry(_) => {
            "(Esc) Back | (x) Lock | (e) Edit | (⌫) Delete | (↵) Copy password | (v) Toggle password visibility"
        }
        AppState::ConfirmEntryDelete => "(Esc) Cancel | (↵) Confirm",
    };
    Line::from(text).dim()
}

fn get_status_text(app: &App) -> Line<'_> {
    match &app.status_message {
        Some(msg) => Line::from(msg.as_str()).dim(),
        None => Line::from(format!("vault v{}", app.vault_version)).dim(),
    }
}

fn render_form(frame: &mut Frame, area: Rect, title_text: &str, app: &App) {
    let [title_area, form_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(80)]));

    let cursor = if is_cursor_visible() { "█" } else { " " };
    
    let mut lines = Vec::new();
    let field_labels = ["Service", "User", "Password"];

    for (i, label) in field_labels.iter().enumerate() {
        let content = &app.form_fields[i];
        let is_active = i == app.current_field;
        
        let line = if is_active {
            Line::from(vec![
                Span::styled(format!("{}: ", label), Style::default().bold()),
                Span::raw(format!("{}{}", content, cursor)),
            ])
        } else {
            Line::from(format!("{}: {}", label, content))
        };
        lines.push(line);
    }

    frame.render_widget(Paragraph::new(title_text).centered(), title_area);
    frame.render_widget(Paragraph::new(lines), form_area);
}

fn render_initial_check(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::bordered().title("Checking vault status..."), area);
}

fn render_unlock_prompt(frame: &mut Frame, area: Rect, input: &str) {
    let horizontal = Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]);
    let [label_area, pass_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&horizontal);

    let label = Line::from("Enter vault password:").bold();
    let cursor = if is_cursor_visible() { "█" } else { " " };
    let mask = "*".repeat(input.chars().count());
    let pass_display = format!("{}{}", mask, cursor); 
    let pass_paragraph = Paragraph::new(pass_display);

    frame.render_widget(label, label_area);
    frame.render_widget(pass_paragraph, pass_area);
}

fn render_locked_vault(frame: &mut Frame, area: Rect) {
    let [area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&Layout::vertical([Constraint::Percentage(100)]));
    frame.render_widget(Line::from("Your vault is locked!").bold().centered(), area);
}

fn render_entries_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows: Vec<Row> = app
        .entries
        .iter()
        .map(|entry| {
            let view = ListEntryView(entry.clone());
            let updated_at = view.updated_at().format("%Y-%m-%d %H:%M").to_string();

            Row::new(vec![
                Cell::from(view.service().to_string()),
                Cell::from(view.username().to_string()),
                Cell::from(updated_at),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Service", "Username/Email", "Last Modified"])
            .style(Style::new().bold().underlined()),
    )
    .style(Style::new().bold())
    .highlight_symbol("|");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_view_entry(frame: &mut Frame, area: Rect, app: &App, view: &DetailEntryView) {
    let [title_area, table_area] = area
        .centered(Constraint::Percentage(80), Constraint::Percentage(60))
        .layout(&Layout::vertical([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ]));

    let Some(detail) = &app.detail_entry else {
        let _ = std::fs::write(
            "blackout_debug.txt",
            "View Entry Error: No entry details available",
        );
        return;
    };

    let pass = if !view.show_password {
        "*".repeat(8)
    } else {
        detail.entry.secret.clone()
    };

    let rows: Vec<_> = vec![
        Row::new(vec![
            Cell::from("Service:"),
            Cell::from(detail.entry.service.clone()),
        ]),
        Row::new(vec![
            Cell::from("Username/Email:"),
            Cell::from(detail.entry.username.clone()),
        ]),
        Row::new(vec![Cell::from("Password:"), Cell::from(pass)]),
        Row::new(vec![
            Cell::from("Last Modified:"),
            Cell::from(detail.entry.updated_at.to_string()),
        ]),
    ]
    .into_iter()
    .map(|r| r.style(Style::new().bold()))
    .collect();

    frame.render_widget(
        Paragraph::new(detail.entry.id.to_string()).centered(),
        title_area,
    );
    frame.render_widget(
        Table::new(rows, [Constraint::Length(20), Constraint::Fill(1)]).column_spacing(2),
        table_area,
    );
}

fn render_delete_confirmation(frame: &mut Frame, area: Rect) {
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
