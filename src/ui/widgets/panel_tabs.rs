use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Tabs},
};

use crate::ui::Draw;

pub(crate) struct PanelTabs {
    selected: bool,
    selected_tab: Option<u16>,
}

impl Draw for PanelTabs {
    fn draw<B: ratatui::prelude::Backend>(
        &mut self,
        f: &mut ratatui::Frame<B>,
        area: ratatui::prelude::Rect,
    ) -> () {
        let (tabs_area, _) = Self::calculate_layout(area);

        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Thick);
        let border = if self.selected { border } else { border.dim() };

        let titles = [" Torrent Info ", " Pieces "]
            .iter()
            .cloned()
            .map(Line::from)
            .collect();

        let tabs = Tabs::new(titles).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Cyan))
                .border_type(BorderType::Thick),
        );

        let tabs = if self.selected { tabs } else { tabs.dim() };

        let tabs = match self.selected_tab {
            Some(n) => tabs
                .select(n.into())
                .highlight_style(Style::default().bg(Color::Green)),
            None => tabs,
        };

        f.render_widget(tabs, tabs_area);
        f.render_widget(border, area);
    }
}

impl PanelTabs {
    pub(crate) fn new() -> Self {
        PanelTabs {
            selected: false,
            selected_tab: None,
        }
    }

    pub(crate) fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.selected_tab = if selected { Some(0) } else { None }
    }

    pub(crate) fn set_tab(&mut self, tab_num: u16) {
        self.selected_tab = Some(tab_num);
    }

    fn calculate_layout(area: Rect) -> (Rect, Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2), Constraint::Ratio(5, 6)])
            .vertical_margin(1)
            .split(area);

        return (layout[0], layout[1]);
    }
}
