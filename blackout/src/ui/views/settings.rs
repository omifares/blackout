use crate::ui::prelude::*;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let [title_area, list_area] = area
        .centered(Constraint::Percentage(50), Constraint::Percentage(50))
        .layout(&Layout::vertical([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ]));

    if let AppState::Settings(ref mut settings) = app.state {
        let items: Vec<ListItem> = settings
            .options
            .iter()
            .map(|opt| ListItem::new(opt.as_str()))
            .collect();

        let list = List::new(items).highlight_symbol("|");

        frame.render_widget(Paragraph::new("Settings").centered(), title_area);
        frame.render_stateful_widget(list, list_area, &mut settings.list_state);
    }
}
