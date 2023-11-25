use ratatui::{
    prelude::{Backend, Rect},
    Frame,
};

mod components;
pub mod controller;
mod widgets;
mod data;

pub trait Draw {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) -> ();
}
