use termion::event::Key;
use tui::layout::Rect;
use tui::terminal::Frame;
use tui::backend::Backend;

pub trait Player {
    fn input(&mut self, key: Key);
    fn tick(&mut self);
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect);
}
