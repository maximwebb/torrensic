use ratatui::{
    prelude::{Alignment, Backend, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::Draw;

pub(crate) struct Title {}

impl Draw for Title {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(Color::Blue)
            ).border_type(BorderType::Thick);
        let title = Paragraph::new("TORRENSIC")
            .bold()
            .alignment(Alignment::Center);

        let layout = Layout::default()
            .horizontal_margin(4)
            .vertical_margin(1)
            .constraints(vec![Constraint::Ratio(1, 1)])
            .split(area)[0];

        f.render_widget(border, area);
        f.render_widget(title, layout);
    }
}
