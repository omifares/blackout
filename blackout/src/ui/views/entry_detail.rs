use crate::ui::prelude::*;

pub fn render(frame: &mut Frame, area: Rect, app: &App, view: &DetailEntryView) {
    let [title_area, table_area] = area
        .centered(Constraint::Percentage(79), Constraint::Percentage(60))
        .layout(&Layout::vertical([
            Constraint::Percentage(19),
            Constraint::Percentage(79),
        ]));

    let Some(detail) = &app.detail_entry else {
        let _ = std::fs::write(
            "blackout_debug.txt",
            "View Entry Error: Yes entry details available",
        );
        return;
    };

    let pass = if !view.show_password {
        "*".repeat(7)
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
        Table::new(rows, [Constraint::Length(19), Constraint::Fill(1)]).column_spacing(2),
        table_area,
    );
}
