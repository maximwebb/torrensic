use bitvec::{prelude::Msb0, vec::BitVec};
use crossterm::{
    self,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    cmp::{max, min},
    error::Error,
    io,
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

use ratatui::{
    backend::CrosstermBackend,
    prelude::{Constraint, Direction, Layout, Rect},
    Terminal,
};

use crate::parser::metadata::Metadata;

use super::{
    components::{title::Title, torrent_progress::TorrentProgress},
    widgets::{
        panel_tabs::PanelTabs, pieces_info::PiecesInfo, torrent_info::TorrentInfo,
        torrent_list::TorrentList,
    },
    Draw,
};

pub(crate) struct Controller {
    pub(crate) md: Arc<Metadata>,
    pub(crate) rx_progress: watch::Receiver<(u32, u32)>,
    pub(crate) rx_pieces: watch::Receiver<BitVec<u8, Msb0>>,
    selected_torrent: u16,
    panel_state: PanelState,
}

impl Controller {
    pub(crate) fn new(
        md: Arc<Metadata>,
        rx_progress: watch::Receiver<(u32, u32)>,
        rx_pieces: watch::Receiver<BitVec<u8, Msb0>>,
    ) -> Self {
        Controller {
            md,
            rx_progress,
            rx_pieces,
            selected_torrent: 0,
            panel_state: PanelState::Hidden,
        }
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut title = Title {};

        let mut torrent_list = TorrentList::new(vec![
            TorrentProgress {
                rx_progress: self.rx_progress.clone(),
                name: (&self.md.info.name).to_string(),
                selected: true,
            },
            TorrentProgress {
                rx_progress: self.rx_progress.clone(),
                name: "Torrent 2".to_string(),
                selected: false,
            },
        ]);

        let mut panel_tabs = PanelTabs::new();

        let torrent_list_len = 2;

        loop {
            terminal.draw(|f| {
                let (title_area, torrent_list_area, tabs_area, tabs_inner_area) =
                    Self::calculate_layout(f.size());

                title.draw(f, title_area);
                torrent_list.draw(f, torrent_list_area);
                panel_tabs.draw(f, tabs_area);

                match &mut self.panel_state {
                    PanelState::Hidden => {}
                    PanelState::TorrentInfo(panel) => {
                        panel.draw(f, tabs_inner_area);
                    }
                    PanelState::PiecesInfo(panel) => {
                        panel.draw(f, tabs_inner_area);
                    }
                }
            })?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if KeyCode::Char('q') == key.code {
                        println!("Quit");
                        break;
                    }
                    match self.panel_state {
                        PanelState::Hidden => {
                            if key.code == KeyCode::Up {
                                self.selected_torrent = max(0, self.selected_torrent - 1);
                                torrent_list.set_torrent(self.selected_torrent);
                            } else if key.code == KeyCode::Down {
                                self.selected_torrent =
                                    min(torrent_list_len - 1, self.selected_torrent + 1);
                                torrent_list.set_torrent(self.selected_torrent);
                            } else if key.code == KeyCode::Right || key.code == KeyCode::Enter {
                                self.panel_state = PanelState::TorrentInfo(TorrentInfo {
                                    md: self.md.clone(),
                                });

                                torrent_list.set_selected(false);
                                panel_tabs.set_selected(true);
                            }
                        }
                        PanelState::TorrentInfo(_) => {
                            if key.code == KeyCode::Esc || key.code == KeyCode::Left {
                                self.panel_state = PanelState::Hidden;
                                torrent_list.set_selected(true);
                                panel_tabs.set_selected(false);
                            } else if key.code == KeyCode::Right {
                                self.panel_state = PanelState::PiecesInfo(PiecesInfo::new(
                                    self.rx_pieces.clone(),
                                ));
                                panel_tabs.set_tab(1);
                            }
                        }
                        PanelState::PiecesInfo(_) => {
                            if key.code == KeyCode::Esc {
                                self.panel_state = PanelState::Hidden;
                                torrent_list.set_selected(true);
                                panel_tabs.set_selected(false);
                            } else if key.code == KeyCode::Left {
                                self.panel_state = PanelState::TorrentInfo(TorrentInfo {
                                    md: self.md.clone(),
                                });
                                panel_tabs.set_tab(0);
                            }
                        }
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    // Splits terminal into following regions: title, torrents list, tabs and tabs inner.
    fn calculate_layout(area: Rect) -> (Rect, Rect, Rect, Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Ratio(5, 6)])
            .split(area);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
            .split(layout[1]);

        let tabs_inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2), Constraint::Ratio(5, 6)])
            .vertical_margin(2)
            .horizontal_margin(3)
            .split(body[1])[1];

        return (layout[0], body[0], body[1], tabs_inner);
    }
}

pub(crate) async fn run_controller_task(mut controller_task: Controller) {
    let _ = controller_task.run().await;
}

enum PanelState {
    Hidden,
    TorrentInfo(TorrentInfo),
    PiecesInfo(PiecesInfo),
}
