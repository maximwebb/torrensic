use ratatui::{
    prelude::{Backend, Buffer, CrosstermBackend, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge},
    Frame,
};
use tokio::sync::watch;

use super::Draw;

pub(crate) struct ProgressBar {
    pub(crate) rx_progress: watch::Receiver<(u32, u32)>,
}

impl Draw for ProgressBar {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, container: Rect) {
        let val = self.rx_progress.borrow();
        let (pieces, total) = *val;

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!("Downloaded {pieces}/{total} pieces"))
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::LightBlue))
            .ratio(ProgressBar::fraction(pieces, total));
        let border = Block::default()
            .title("Torrent progress")
            .borders(Borders::ALL);

        let area = Self::area(container);

        f.render_widget(border, area);
        f.render_widget(gauge, Self::bar_area(area));
    }
}

impl ProgressBar {
    fn fraction(pieces: u32, total: u32) -> f64 {
        if total == 0 {
            return 0f64;
        }
        let div = (pieces as f64) / (total as f64);
        return (div * 100.0).round() / 100.0;
    }

    fn area(container: Rect) -> Rect {
        Rect {
            x: container.x + 2,
            y: container.y + 2,
            width: container.width / 2,
            height: container.height / 2,
        }
    }

    fn bar_area(container: Rect) -> Rect {
        Rect {
            x: container.x + 2,
            y: (container.y + container.height) / 2,
            width: container.width - 4,
            height: 3,
        }
    }
}
