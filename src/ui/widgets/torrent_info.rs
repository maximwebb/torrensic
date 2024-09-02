use std::sync::Arc;

use ratatui::{
    prelude::{Backend, Rect},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::{parser::bootstrap_info::BootstrapInfo, ui::Draw};

pub(crate) struct TorrentInfo {
    pub(crate) md: Arc<BootstrapInfo>,
}

impl Draw for TorrentInfo {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let lines = vec![
            Line::from(format!("Name: {}", self.md.info.name)),
            Line::from(""),
            Line::from(format!(
                "Total size: {:.2}Mb",
                ((u32::try_from(self.md.num_pieces()).unwrap() * self.md.info.piece_length) as f32)
                    / 1000000f32
            )),
            Line::from(""),
            Line::from(format!("Pieces: {}", self.md.num_pieces())),
            Line::from(""),
            Line::from(format!("Tracker: {}", self.md.announce_list[0].join(""))),
        ];

        let text = Paragraph::new(lines);
        f.render_widget(text, area);
    }
}
