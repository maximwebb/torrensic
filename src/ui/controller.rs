use crossterm::{
    self,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, time::Duration};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::timeout,
};

use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    widgets::{Block, Borders},
    Terminal,
};

use super::widgets::{progress_bar::ProgressBar, Draw};

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
        let mut progress_bars = vec![ProgressBar {
            rx_progress: self.rx_progress.clone(),
        }];
        loop {
            terminal.draw(|f| {
                let size = f.size();

                let block = Block::default().title("Torrensic").borders(Borders::ALL);
                f.render_widget(block, size);
                progress_bars[0].draw(f, size);
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

    async fn get_status(&mut self) -> (u32, u32) {
        *self.rx_progress.borrow()
    }
}

pub(crate) async fn run_controller_task(mut controller_task: Controller) {
    let _ = controller_task.run().await;
}
