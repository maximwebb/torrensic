use crossterm::{
    self,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};

use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    widgets::{Block, Borders},
    Terminal,
};

use crate::client::peer_manager::StatusRequest;

use super::widgets::progress::Progress;

pub(crate) struct Controller {
    pub(crate) sender: mpsc::Sender<StatusRequest>,
}

impl Controller {
    pub(crate) fn new(sender: mpsc::Sender<StatusRequest>) -> Self {
        Controller { sender }
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let progress = Progress {};
        loop {
            let (pieces, total) = match self.get_status().await {
                Some(pair) => pair,
                None => (0, 0),
            };

            terminal.draw(|f| {
                let size = f.size();
                let progress_size = Rect {
                    x: size.x + 2,
                    y: size.y + 2,
                    width: size.width / 2,
                    height: size.height / 2,
                };
                let block = Block::default().title("Torrensic").borders(Borders::ALL);
                f.render_widget(block, size);
                progress.draw(f, progress_size, pieces, total);
            })?;
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if KeyCode::Char('q') == key.code {
                        println!("Quit");
                        break;
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

    async fn get_status(&mut self) -> Option<(u32, u32)> {
        let (chan, recv) = oneshot::channel();
        let _ = self.sender.send(StatusRequest { chan }).await;

        let status = match timeout(Duration::from_millis(200), recv).await {
            Ok(res) => res,
            Err(_) => return None,
        };

        return match status {
            Ok(pair) => Some(pair),
            Err(_) => None,
        };
    }
}

pub(crate) async fn run_controller_task(mut controller_task: Controller) {
    let _ = controller_task.run().await;
}
