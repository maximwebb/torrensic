use bitvec::{prelude::Msb0, vec::BitVec};
use ratatui::{
    prelude::{Backend, Rect},
    style::Color,
    symbols::Marker,
    widgets::canvas::{Canvas, Painter, Shape},
    Frame,
};
use tokio::sync::watch;

use crate::ui::Draw;

pub(crate) struct PiecesInfo {
    pub(crate) rx_pieces: watch::Receiver<BitVec<u8, Msb0>>,
    buf: BitVec<u8, Msb0>,
    width: u16,
}

impl Draw for PiecesInfo {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        self.buf = self.rx_pieces.borrow().to_owned();
        self.width = f.size().width;

        let canvas = Canvas::default()
            .marker(Marker::Block)
            .paint(|ctx| ctx.draw(self));
        f.render_widget(canvas, area);
    }
}

impl Shape for PiecesInfo {
    fn draw(&self, painter: &mut Painter) {
        for i in 0..self.buf.len() {
            let color = if self.buf[i] {
                Color::LightGreen
            } else {
                Color::LightRed
            };
            let (x, y) = (i % (self.width as usize), i / (self.width as usize));
            painter.paint(x, y, color);
        }
    }
}

impl PiecesInfo {
    pub(crate) fn new(rx_pieces: watch::Receiver<BitVec<u8, Msb0>>) -> Self {
        let buf = rx_pieces.borrow().to_owned();
        PiecesInfo {
            rx_pieces,
            buf,
            width: 20,
        }
    }
}