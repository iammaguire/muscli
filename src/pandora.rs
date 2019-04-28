use pandora_rs2::Pandora;

use rfmod::Sys;
use std::io;
use termion::event::Key;
use tui::Terminal;
use tui::backend::Backend;
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::terminal::Frame;
use super::player::Player;
use super::Config;

pub struct PandoraPlayer {
    config: Config,
    handle: Pandora
}

impl PandoraPlayer {
    pub fn new(config: Config) -> PandoraPlayer {
        PandoraPlayer {
            handle: Pandora::new(&config.username, &config.password).expect("Couldn't initialize pandora."),
            config: config
        }
    }
}

impl Player for PandoraPlayer {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
    }
    fn input(&mut self, key: Key, fmod: &Sys) {}
    fn tick(&mut self, fmod: &Sys) {}
}