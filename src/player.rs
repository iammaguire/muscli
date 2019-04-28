use termion::event::Key;
use tui::layout::Rect;
use tui::terminal::Frame;
use tui::backend::Backend;
use rfmod::Sys;

pub trait Player {
    fn input(&mut self, key: Key, fmod: &Sys);
    fn tick(&mut self, fmod: &Sys);
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect);
}
