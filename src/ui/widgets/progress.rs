use ratatui::{
    prelude::{Backend, Buffer, CrosstermBackend, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Widget},
    Frame,
};

pub(crate) struct Progress {}

impl Progress {
    pub(crate) fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect, pieces: u32, total: u32) {
        let bar_size = Rect {
            x: area.x + 2,
            y: (area.y + area.height) / 2,
            width: area.width - 4,
            height: 3,
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!("Downloaded {pieces}/{total} pieces"))
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::LightBlue))
            .ratio(Progress::fraction(pieces, total));
        let border = Block::default()
            .title("Torrent progress")
            .borders(Borders::ALL);

        f.render_widget(border, area);
        f.render_widget(gauge, bar_size);
    }

    fn fraction(pieces: u32, total: u32) -> f64 {
        if total == 0 {
            return 0f64;
        }
        let div = (pieces as f64) / (total as f64);
        return (div * 100.0).round() / 100.0;
    }
}
