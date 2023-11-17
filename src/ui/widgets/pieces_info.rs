use bitvec::{prelude::Msb0, vec::BitVec};
use ratatui::{
    prelude::{Backend, Constraint, Layout, Rect},
    style::Color,
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Painter, Shape},
        Paragraph,
    },
    Frame,
};
use tokio::sync::watch;

use crate::ui::Draw;

pub(crate) struct PiecesInfo {
    pub(crate) rx_in_progress_pieces: watch::Receiver<BitVec<u8, Msb0>>,
    pub(crate) rx_downloaded_pieces: watch::Receiver<BitVec<u8, Msb0>>,
    in_progress_pieces: BitVec<u8, Msb0>,
    downloaded_pieces: BitVec<u8, Msb0>,
    width: u16,
}

impl Draw for PiecesInfo {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        self.in_progress_pieces = self.rx_in_progress_pieces.borrow().to_owned();
        self.downloaded_pieces = self.rx_downloaded_pieces.borrow().to_owned();
        self.width = f.size().width / 2;

        let (text_area, heatmap_area) = Self::calculate_layout(area);

        let text = Paragraph::new(format!(
            "Downloaded {}/{} pieces.",
            self.downloaded_pieces.count_ones(),
            self.downloaded_pieces.len()
        ));

        let canvas = Canvas::default()
            .marker(Marker::Block)
            .x_bounds([0.0, self.width.into()])
            .y_bounds([0.0, self.width.into()])
            .paint(|ctx| ctx.draw(self));

        f.render_widget(text, text_area);
        f.render_widget(canvas, heatmap_area);
    }
}

impl Shape for PiecesInfo {
    fn draw(&self, painter: &mut Painter) {
        for i in 0..self.downloaded_pieces.len() {
            let color = if self.downloaded_pieces[i] {
                Color::LightGreen
            } else if self.in_progress_pieces[i] {
                Color::Yellow
            }
            else {
                Color::LightRed
            };
            let (x, y) = (i % (self.width as usize), i / (self.width as usize));
            painter.paint(x, y, color);
        }
    }
}

impl PiecesInfo {
    pub(crate) fn new(rx_in_progress_pieces: watch::Receiver<BitVec<u8, Msb0>>, rx_downloaded_pieces: watch::Receiver<BitVec<u8, Msb0>>) -> Self {
        let in_progress_pieces = rx_in_progress_pieces.borrow().to_owned();
        let downloaded_pieces = rx_downloaded_pieces.borrow().to_owned();
        PiecesInfo {
            rx_in_progress_pieces,
            rx_downloaded_pieces,
            in_progress_pieces,
            downloaded_pieces,
            width: 20,
        }
    }

    fn calculate_layout(area: Rect) -> (Rect, Rect) {
        let layout = Layout::default()
            .constraints(vec![(Constraint::Length(2)), (Constraint::Min(1))])
            .split(area);

        return (layout[0], layout[1]);
    }
}
