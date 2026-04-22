use crate::ui::prelude::*;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
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
