use ratatui::{
    prelude::{Alignment, Backend, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols, text,
    widgets::{Block, BorderType, Borders, LineGauge, Paragraph},
    Frame,
};
use tokio::sync::watch;

use crate::ui::Draw;

pub(crate) struct TorrentProgress {
    pub(crate) rx_progress: watch::Receiver<(u32, u32)>,
    pub(crate) name: String,
    pub(crate) selected: bool,
}

impl Draw for TorrentProgress {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let val = self.rx_progress.borrow();
        let (pieces, total) = *val;
        let (text_area, line_area) = Self::calculate_layout(area);

        let border = Block::default()
            .title_alignment(Alignment::Right)
            .borders(Borders::ALL);
        let text = Paragraph::new(vec![
            text::Line::from(format!("{} - {pieces}/{total} pieces", self.name)),
            text::Line::from(""),
        ]);
        let line_gauge = LineGauge::default()
            .gauge_style(Style::default().fg(Color::Magenta))
            .ratio(TorrentProgress::fraction(pieces, total));

        let border = if self.selected {
            border.bold()
        } else {
            border.dim()
        };
        let text = if self.selected { text } else { text.dim() };
        let line_gauge = if self.selected {
            line_gauge.line_set(symbols::line::THICK)
        } else {
            line_gauge
        };

        f.render_widget(border, area);
        f.render_widget(text, text_area);
        f.render_widget(line_gauge, line_area);
    }
}

impl TorrentProgress {
    pub(crate) fn set_selected(&mut self, select: bool) {
        self.selected = select;
    }

    fn calculate_layout(area: Rect) -> (Rect, Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .horizontal_margin(2)
            .vertical_margin(1)
            .split(area);
        return (layout[0], layout[1]);
    }

    fn fraction(pieces: u32, total: u32) -> f64 {
        if total == 0 {
            return 0f64;
        }
        let div = (pieces as f64) / (total as f64);
        return (div * 100.0).round() / 100.0;
    }
}
