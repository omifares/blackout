use crate::ui::prelude::*;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows: Vec<Row> = app
        .snapshots
        .iter()
        .rev()
        .map(|shot| {
            let shot_view = SnapshotView {
                version: shot.version,
                created_at: shot.created_at,
                checksum: shot.checksum.clone(),
                reason: shot.reason.clone(),
            };

            let display_hash = shot_view
                .checksum
                .get(..7)
                .unwrap_or(&shot_view.checksum)
                .to_string();

            Row::new(vec![
                Cell::from(shot_view.version.to_string()),
                Cell::from(display_hash),
                Cell::from(shot_view.created_at.format("%Y-%m-%d %H:%M").to_string()),
                Cell::from(shot_view.reason.to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Fill(1),
        ],
    )
    .header(
        Row::new(vec!["version", "Checksum", "Created at", "Reason"])
            .style(Style::new().bold().underlined()),
    )
    .style(Style::new().bold())
    .highlight_symbol("|");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}
