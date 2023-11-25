use ratatui::{
    prelude::{Alignment, Backend, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::{components::torrent_progress::TorrentProgress, Draw};

pub(crate) struct TorrentList {
    pub(crate) torrents: Vec<TorrentProgress>,
    selected: bool,
}


impl Draw for TorrentList {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let (title_area, body_areas) = Self::calculate_layout(self.torrents.len(), area);

        let border_type = if self.selected {
            BorderType::Thick
        } else {
            BorderType::Plain
        };

        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(border_type);
        let title = Paragraph::new("TORRENT PROGRESS")
            .bold()
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::Cyan))
                    .border_type(border_type),
            );

        f.render_widget(title, title_area);
        f.render_widget(border, area);

        for (torrent, size) in self.torrents.iter_mut().zip(body_areas.iter()) {
            torrent.draw(f, *size);
        }
    }
}

impl TorrentList {
    pub(crate) fn new(torrents: Vec<TorrentProgress>) -> Self {
        TorrentList {
            torrents,
            selected: true,
        }
    }

    pub(crate) fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub(crate) fn set_torrent(&mut self, torrent_num: u16) {
        let torrent_num = torrent_num as usize;
        for (i, torrent) in self.torrents.iter_mut().enumerate() {
            torrent.set_selected(i == torrent_num);
        }
    }

    fn calculate_layout(num: usize, area: Rect) -> (Rect, Vec<Rect>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2), Constraint::Ratio(5, 6)])
            .vertical_margin(1)
            .split(area);

        let (title, body) = (layout[0], layout[1]);

        let constraints = [vec![Constraint::Length(5); num], vec![Constraint::Max(100)]].concat();
        let body = Layout::default()
            .constraints(constraints)
            .horizontal_margin(2)
            .vertical_margin(0)
            .split(body);

        return (title, body[..body.len() - 1].to_vec());
    }
}
