use ratatui::{prelude::{Backend, Rect}, Frame};

pub mod progress_bar;

pub trait Draw {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, container: Rect) -> (); 
}