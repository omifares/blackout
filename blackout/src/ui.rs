use ratatui::widgets::{Block, Cell, Paragraph, Row, Table};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
};

use crate::app::{App, AppState, DetailEntryView, EntryView, ListEntryView};

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
    let [top, main, status_area, bottom] = frame.area().layout(&vertical);

    let title = Line::from_iter([Span::from("Blackout").bold()]);
    frame.render_widget(title.centered(), top);

    match &app.state {
        AppState::InitialCheck => render_initial_check(frame, main),
        AppState::UnlockPrompt => render_unlock_prompt(frame, main, &app.input_buffer),
        AppState::VaultLocked => render_locked_vault(frame, main),
        AppState::EntriesList => render_entries_list(frame, main, app),
        AppState::NewEntryForm => render_new_entry_form(frame, main, app),
        AppState::ViewEntry(view) => render_view_entry(frame, main, app, &view),
        AppState::UpdateEntry => render_edit_entry_form(frame, main, app),
        AppState::ConfirmEntryDelete => render_delete_confirmation(frame, main),
    }

    let helper = match &app.state {
        AppState::InitialCheck => Line::from("(Esc) Quit").dim(),
        AppState::UnlockPrompt => Line::from("(Enter) Submit | (Esc) Quit").dim(),
        AppState::VaultLocked => Line::from("(Esc) Quit | (any) Unlock vault").dim(),
        AppState::EntriesList => {
            Line::from("(Esc) Quit | (h) Help | (↵) Select | (⌫) Delete | (n) New | (x) Lock").dim()
        }
        AppState::NewEntryForm => {
            Line::from("(Tab) Next field | (BackTab) Prev field | (Enter) Submit | (Esc) Cancel")
                .dim()
        }
        AppState::ViewEntry(_view) => {
            Line::from("(Esc) Back | (x) Lock | (e) Edit | (⌫) Delete | (↵) Copy password | (v) Toggle password visibility")
                .dim()
        }
        AppState::UpdateEntry => {
            Line::from("(Tab) Next field | (BackTab) Prev field | (Enter) Submit | (Esc) Cancel")
                .dim()
        }
        AppState::ConfirmEntryDelete => {
            Line::from("(Esc) Cancel | (↵) Confirm ")
                .dim()
        }
    };

    let status = match &app.state {
        AppState::InitialCheck => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::UnlockPrompt => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::VaultLocked => {Line::from(app.status_message.clone().unwrap_or_default()).dim().dim()},
        AppState::EntriesList => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::NewEntryForm => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::ViewEntry(_view) => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::UpdateEntry => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
        AppState::ConfirmEntryDelete => { Line::from(app.status_message.clone().unwrap_or_default()).dim() },
    };

    frame.render_widget(status.centered(), status_area);
    frame.render_widget(helper.centered(), bottom);
}

fn render_initial_check(frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title("Checking vault status...");
    frame.render_widget(block, area);
}

fn render_unlock_prompt(frame: &mut Frame, area: Rect, input: &str) {
    let horizontal = Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]);
    let [label_area, pass_area] = area
        .centered(Constraint::Percentage(20), Constraint::Percentage(20))
        .layout(&horizontal);

    let label = Line::from("Enter vault password:").bold();
    let pass: String = "*".repeat(input.chars().count());
    let pass_paragraph = Paragraph::new(pass);

    frame.render_widget(label, label_area);
    frame.render_widget(pass_paragraph, pass_area);
}

fn render_locked_vault(frame: &mut Frame, area: Rect) {
    let vertical = Layout::vertical([Constraint::Percentage(100)]);
    let [label_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(20))
        .layout(&vertical);

    let label = Line::from("Your vault is locked!").bold().centered();

    frame.render_widget(label, label_area);
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
            .style(Style::new()) 
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

fn render_new_entry_form(frame: &mut Frame, area: Rect, app: &App) {
    let vertical = Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(80)]);
    let [title_area, form_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&vertical);

    let title = Paragraph::new("New entry").centered();
    let text = format!(
        "Service: {}\nUser: {}\nPassword: {}",
        app.form_fields[0], app.form_fields[1], app.form_fields[2]
    );
    let form = Paragraph::new(text);

    frame.render_widget(title, title_area);
    frame.render_widget(form, form_area);
}

fn render_view_entry(frame: &mut Frame, area: Rect, app: &App, view: &DetailEntryView) {
    let vertical = Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(80)]);
    let [title_area, table_area] = area
        .centered(Constraint::Percentage(80), Constraint::Percentage(60))
        .layout(&vertical);

    if app.detail_entry.is_none() {
        let debug_info = "View Entry Error: No entry details available".to_string();
        let _ = std::fs::write("blackout_debug.txt", debug_info);
        return;
    }

    let pass = if !view.show_password { "*".repeat(8) } else { app.detail_entry.as_ref().unwrap().entry.secret.to_string() };

    let rows = vec![
        Row::new(vec![
            Cell::from("Service:"),
            Cell::from(app.detail_entry.as_ref().unwrap().entry.service.clone()),
        ]),
        Row::new(vec![
            Cell::from("Username/Email:"),
            Cell::from(app.detail_entry.as_ref().unwrap().entry.username.clone()),
        ]),
        Row::new(vec![
            Cell::from("Password:"),
            Cell::from(pass),
        ]),
        Row::new(vec![
            Cell::from("Last Modified:"),
            Cell::from(app.detail_entry.as_ref().unwrap().entry.updated_at.to_string()),
        ]),
    ];

    let rows: Vec<Row> = rows
        .into_iter()
        .map(|row| row.style(Style::new().bold()))
        .collect();

    let table = Table::new(
        rows, 
        [
            Constraint::Length(20),
            Constraint::Fill(1),
        ]
    )
    .column_spacing(2);

    let title = Paragraph::new(app.detail_entry.as_ref().unwrap().entry.id.to_string()).centered();
    
    frame.render_widget(title, title_area);
    frame.render_widget(table, table_area);
}

fn render_edit_entry_form(frame: &mut Frame, area: Rect, app: &App) {
    let vertical = Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(80)]);
    let [title_area, form_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&vertical);

    let title = Paragraph::new("Edit entry").centered();
    let text = format!(
        "Service: {}\nUser: {}\nPassword: {}",
        app.form_fields[0], app.form_fields[1], app.form_fields[2]
    );
    let form = Paragraph::new(text);

    frame.render_widget(title, title_area);
    frame.render_widget(form, form_area);
}

fn render_delete_confirmation(frame: &mut Frame, area: Rect) {
    let vertical = Layout::vertical([Constraint::Percentage(50)]);
    let [confirm_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(60))
        .layout(&vertical);

    let text = vec![
        Line::from("Are you sure you want to delete this entry?").centered(),
        Line::from(""),
        Line::from(" [y]es  |  [n]o ").centered().bold(),
    ];

    let paragraph = Paragraph::new(text).centered();

    frame.render_widget(paragraph, confirm_area);
}
