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
    time::Duration,
};
use tokio::sync::watch;

use ratatui::{
    backend::CrosstermBackend,
    prelude::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Terminal,
};

use super::{
    components::{title::Title, torrent_progress::TorrentProgress},
    widgets::torrent_list::TorrentList,
    Draw,
};

pub(crate) struct Controller {
    pub(crate) rx_progress: watch::Receiver<(u32, u32)>,
}

impl Controller {
    pub(crate) fn new(rx_progress: watch::Receiver<(u32, u32)>) -> Self {
        Controller { rx_progress }
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut title = Title {};

        let mut torrent_list = TorrentList {
            torrents: vec![
                TorrentProgress {
                    rx_progress: self.rx_progress.clone(),
                    name: "Torrent 1".to_string(),
                    selected: true,
                },
                TorrentProgress {
                    rx_progress: self.rx_progress.clone(),
                    name: "Torrent 2".to_string(),
                    selected: false,
                },
            ],
        };

        let mut selected_torrent: u16 = 0;
        let torrent_list_len = 2;

        loop {
            terminal.draw(|f| {
                let (title_layout, body_layout) = Self::calculate_layout(f.size());

                title.draw(f, title_layout);
                torrent_list.draw(f, body_layout[0]);
            })?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if KeyCode::Char('q') == key.code {
                        println!("Quit");
                        break;
                    } else if KeyCode::Up == key.code {
                        selected_torrent = max(0, selected_torrent - 1);
                        torrent_list.set_torrent(selected_torrent);
                    } else if KeyCode::Down == key.code {
                        selected_torrent = min(torrent_list_len - 1, selected_torrent + 1);
                        torrent_list.set_torrent(selected_torrent);
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

    fn calculate_layout(area: Rect) -> (Rect, Vec<Rect>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Ratio(5, 6)])
            .split(area);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
            .split(layout[1]);

        return (layout[0], body.to_vec());
    }
}

pub(crate) async fn run_controller_task(mut controller_task: Controller) {
    let _ = controller_task.run().await;
}
