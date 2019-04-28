extern crate rfmod;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate dirs;
extern crate base64;

pub mod util;
pub mod event;
pub mod pandora;
pub mod local;
pub mod player;

use std::env;
use std::io;
use std::fs::File;
use std::path::Path;
use std::iter;
use termion::raw::{ RawTerminal, IntoRawMode };
use termion::event::Key;
use tui::Terminal;
use tui::backend::{ Backend, TermionBackend };
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text, Tabs };
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::style::{ Color, Modifier, Style};
use tui::terminal::Frame;
use event::{ Event, Events };
use util::TabsState;
use pandora::PandoraPlayer;
use local::LocalPlayer;
use player::Player;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    username: String,
    password: String
}

pub struct App<'a> {
    tabs: TabsState<'a>,
    pandora_player: Box<PandoraPlayer<'a>>,
    local_player: Box<LocalPlayer<'a>>,
}

// hardcoded input/tick/draw calls because can't figure out how to return a trait.. jfc
impl<'a> App<'a> {
    fn input(&mut self, key: Key) {
        if self.tabs.index == 0 {
            self.local_player.input(key);
        } else {
            self.pandora_player.input(key);
        }
    }

    fn tick(&mut self) {
        if self.tabs.index == 0 {
            self.local_player.tick();
        } else {
            self.pandora_player.tick();
        }
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if self.tabs.index == 0 {
            self.local_player.draw(f, chunk);
        } else {
            self.pandora_player.draw(f, chunk);
        }
    }
}

fn main() -> Result<(), failure::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let events = Events::new();
    let config = read_config();

    let fmod = match rfmod::Sys::new() {
        Ok(f) => f,
        Err(e) => {
            panic!("Error code : {:?}", e);
        }
    };

    match fmod.init() {
        rfmod::Status::Ok => {}
        e => {
            panic!("FmodSys.init failed : {:?}", e);
        }
    };

    let mut app = App {
        tabs: TabsState::new(vec!["Local", "Pandora", "Spotify"]),
        pandora_player: Box::new(PandoraPlayer::new(&config)),
        local_player: Box::new(LocalPlayer::new(&config, &fmod))
    };

    loop {
        // User input
        match events.next()? {
            Event::Input(input) => match input {
                Key::Right => app.tabs.next(),
                Key::Left => app.tabs.previous(),
                Key::Char('q') => { break; },
                _ => app.input(input)
            }
            Event::Tick => app.tick()
        }

        // Draw UI
        terminal.draw(|mut f| {
            let root_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Percentage(15), Constraint::Percentage(85)])
                .split(f.size());
            Tabs::default()
                .block(Block::default().borders(Borders::ALL).title("Interfaces"))
                .titles(&app.tabs.titles)
                .select(app.tabs.index)
                .style(Style::default().modifier(Modifier::ITALIC))
                .render(&mut f, root_chunks[0]);

            app.draw(&mut f, root_chunks[1]);
        }).unwrap();
    }
    Ok(())
}

fn read_config() -> Config {
    let mut config_file_path = dirs::config_dir().expect("Config directory couldn't be found.");
    config_file_path.push("muscli/config.json");
    let config_file = File::open(config_file_path).expect("Config file not found");
    let mut config: Config = serde_json::from_reader(config_file).expect("Error while reading config file");
    config.password = String::from_utf8(base64::decode(&config.password).unwrap()).unwrap();
    config.password.pop(); // remove trailing byte may be unnecessary
    config
}
