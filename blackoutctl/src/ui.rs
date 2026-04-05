use ratatui::{Frame, layout::{Constraint, Layout, Rect}, macros::horizontal, style::{Color, Style, Stylize}, text::{Line, Span}, widgets::Padding};
use ratatui::widgets::{Table, Row, Cell, Block, BorderType, Paragraph};

use crate::app::{App, EntryView, ListEntryView, AppState};

pub fn render(frame: &mut Frame, app: &App) {
    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])
    .spacing(1)
    .margin(3);
    let [top, main, bottom] = frame.area().layout(&vertical);

    let title = Line::from_iter([
        Span::from("Blackout").bold(),
    ]);
    frame.render_widget(title.centered(), top);

    match app.state {
        AppState::InitialCheck => render_initial_check(frame, main),
        AppState::UnlockPrompt => render_unlock_prompt(frame, main, &app.input_buffer),
        AppState::EntriesList => render_entries_list(frame, main, app),
        AppState::NewEntryForm => render_new_entry_form(frame, main, app),
        AppState::ViewEntry(_) => render_view_entry(frame, main, app, ),
    }

    let helper = match app.state {
        AppState::InitialCheck => Line::from("(q) Quit").dim(),
        AppState::UnlockPrompt => Line::from("(Enter) Submit | (q) Quit").dim(),
        AppState::EntriesList => Line::from("(q) Quit | (h) Help | (↵) Select | (⌫) Delete | (n) New | (x) Lock and quit").dim(),
        AppState::NewEntryForm => Line::from("(Tab) Next field | (Enter) Submit | (Esc) Cancel").dim(),
        AppState::ViewEntry(_) => Line::from("(Esc) Back").dim(),
    };
    frame.render_widget(helper.centered(), bottom);
}

fn render_initial_check(frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title("Checking vault status...");
    frame.render_widget(block, area);
}

fn render_unlock_prompt(frame: &mut Frame, area: Rect, input: &str) {
    let horizontal = Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]);
    let [ label_area, pass_area] = area.centered(Constraint::Percentage(50), Constraint::Percentage(50)).layout(&horizontal);

    let label = Line::from("Enter vault password:").bold();
    let pass: String = "*".repeat(input.chars().count());
    let pass_paragraph = Paragraph::new(pass);

    frame.render_widget(label, label_area);
    frame.render_widget(pass_paragraph, pass_area);
}

fn render_entries_list(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = app.entries.iter().enumerate().map(|(i, entry)| {
        let view = ListEntryView(entry.clone()); 
        
        let style = if i == app.selected_entry {
            Style::new().bold()
        } else {
            Style::new().bold().fg(Color::Yellow)
        };

        Row::new(vec![
            Cell::from(view.service().to_string()),
            Cell::from(view.username().to_string()),
            Cell::from(view.updated_at().to_string()),
        ])
        .style(style)
    }).collect();
    
    let table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(40),
        Constraint::Percentage(20),
    ])
    .header(Row::new(vec!["Service", "Username/Email", "Last Modified"])
    .style(Style::new().bold().underlined()));

    frame.render_widget(table, area);
}

fn render_new_entry_form(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::new()
        .title("New Entry");
    let text = format!(
        "Service: {}\nUser: {}\nPassword: {}",
        app.form_fields[0], app.form_fields[1], app.form_fields[2]
    );
    let para = Paragraph::new(text).block(block);
    frame.render_widget(para, area);
}

fn render_view_entry(frame: &mut Frame, area: Rect, app: &App) {
    if let AppState::ViewEntry(ref view) = app.state {
        let block = Block::bordered()
            .title("View Entry");
            
        let text = format!(
            "Service: {}\nUser: {}\nPassword: {}",
            view.service(), view.username(), view.secret()
        );
        let para = Paragraph::new(text).block(block);
        frame.render_widget(para, area);
    } else {
        let block = Block::bordered().title("View Entry");
        let para = Paragraph::new("No entry selected").block(block);
        frame.render_widget(para, area);
    }
}
