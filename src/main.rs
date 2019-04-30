extern crate rfmod;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate dirs;
extern crate base64;
extern crate reqwest; 

pub mod util;
pub mod event;
pub mod pandora;
pub mod local;
pub mod player;

use std::io;
use std::fs::File;
use rfmod::Sys;
use termion::raw::{ IntoRawMode };
use termion::event::Key;
use tui::Terminal;
use tui::backend::{ Backend, TermionBackend };
use tui::widgets::{ Widget, Block, Borders, Tabs };
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::style::{ Modifier, Style};
use tui::terminal::Frame;
use event::{ Event, Events };
use util::TabsState;
use pandora::PandoraPlayer;
use local::LocalPlayer;
use player::{ Player, MediaUI };

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    username: String,
    password: String
}

pub struct App<'a> {
    tabs: TabsState<'a>,
    pandora_player: PandoraPlayer,
    local_player: LocalPlayer,
    config: Config,
    fmod: Sys,
}

// hardcoded input/tick/draw calls because can't figure out how to return a trait object.. jfc
impl<'a> App<'a> {
    fn new() -> App<'a> {
        let config = App::read_config();

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

        App {
            tabs: TabsState::new(vec!["Local", "Pandora", "Spotify"]),
            pandora_player: PandoraPlayer::new(config.clone()),
            local_player: LocalPlayer::new(config.clone()),
            config: config,
            fmod: fmod
        }
    }
    
    fn read_config() -> Config {
        let mut config_file_path = dirs::config_dir().expect("Config directory couldn't be found.");
        config_file_path.push("muscli/config.json");
        let config_file = File::open(config_file_path).expect("Config file not found");
        let mut config: Config = serde_json::from_reader(config_file).expect("Error while reading config file");
        config.password = String::from_utf8(base64::decode(&config.password).unwrap()).unwrap();
        config.password.pop(); // remove trailing byte, may be unnecessary
        config
    }

    fn input(&mut self, key: Key) {
        if self.tabs.index == 0 {
            self.local_player.input(key, &self.fmod);
        } else {
            self.pandora_player.input(key, &self.fmod);
        }
    }

    fn tick(&mut self) {
        if self.tabs.index == 0 {
            self.local_player.tick(&self.fmod);
        } else {
            self.pandora_player.tick(&self.fmod);
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
    let mut app = App::new();
    let events = Events::new();
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    loop {
        match events.next()? {
            Event::Input(input) => match input {
                Key::Right => app.tabs.next(),
                Key::Left => app.tabs.previous(),
                Key::Char('q') => { break; },
                _ => app.input(input)
            }
            Event::Tick => app.tick()
        }

        terminal.draw(|mut f| {
            let root_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Percentage(15), Constraint::Percentage(85)])
                .split(f.size());
            Tabs::default()
                .block(Block::default().borders(Borders::ALL).title("Interface"))
                .titles(&app.tabs.titles)
                .select(app.tabs.index)
                .style(Style::default().modifier(Modifier::ITALIC))
                .render(&mut f, root_chunks[0]);

            app.draw(&mut f, root_chunks[1]);
        }).unwrap();
    }
    Ok(())
}