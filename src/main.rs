extern crate rfmod;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate dirs;
extern crate base64;
extern crate reqwest; 
extern crate rspotify;

pub mod util;
pub mod event;
pub mod pandora; 
pub mod local;
pub mod player;
pub mod dir_select;
pub mod lyrics;
pub mod spotify;

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
use player::{ Player, MediaPlayer };
use dir_select::DirSelect;
use lyrics::LyricsGrabber;
use spotify::SpotifyPlayer;

pub const DIR_GUI_CODE:     usize = 444;
pub const LOCAL_GUI_CODE:   usize = 0;
pub const PANDORA_GUI_CODE: usize = 1;
pub const SPOTIFY_GUI_CODE: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    local_dir: String,
    pandora_username: String,
    pandora_password: String,
    genius_token: String
}

// Massive TODO: refactor to composition pattern
pub struct App<'a> {
    tabs: TabsState<'a>,
    pandora_player: PandoraPlayer,
    local_player: LocalPlayer,
    media_player: MediaPlayer,
    dir_select: Option<DirSelect>,
    config: Config,
    fmod: Sys
}

// hardcoded input/tick/draw calls because can't figure out how to return a trait object.. stuck in an inheritance mindset
impl<'a> App<'a> {
    fn new() -> App<'a> {
        let config = App::read_config();
        //let spotify = SpotifyPlayer::new(config.clone());
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
            media_player: MediaPlayer::new(config.clone()),
            dir_select: Some(DirSelect::new()),
            config: config,
            fmod: fmod,
        }
    }

    pub fn rebuild_local_with_dir(&mut self, path: &String) {
        self.config.local_dir = path.clone();
        self.local_player = LocalPlayer::new(self.config.clone());
        self.tabs.index = LOCAL_GUI_CODE;
    }
    
    fn read_config() -> Config { // TODO add error obj to Config describing failure and show in whichever interface is affected. Also move pandora pass logic to Pandora
        let mut config_file_path = dirs::config_dir().expect("Config directory couldn't be found.");
        config_file_path.push("muscli/config.json");
        let config_file = File::open(config_file_path).expect("Config file not found");
        let mut config: Config = serde_json::from_reader(config_file).expect("Error while reading config file");
        config.pandora_password = String::from_utf8(base64::decode(&config.pandora_password).unwrap()).unwrap();
        config
    }

    fn input(&mut self, key: Key) {
        match self.tabs.index {
            LOCAL_GUI_CODE => { self.local_player.input(key, &self.fmod, &mut self.media_player); }
            PANDORA_GUI_CODE => { self.pandora_player.input(key, &self.fmod, &mut self.media_player); }
            SPOTIFY_GUI_CODE => {}
            DIR_GUI_CODE => { 
                let mut dir_select = self.dir_select.clone().unwrap();
                dir_select.input(key, self);
                self.dir_select = Some(dir_select);
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        match self.tabs.index {
            LOCAL_GUI_CODE => { self.local_player.tick(&self.fmod, &mut self.media_player); }
            PANDORA_GUI_CODE => { self.pandora_player.tick(&self.fmod, &mut self.media_player); }         
            SPOTIFY_GUI_CODE => {}
            DIR_GUI_CODE => { self.dir_select.as_mut().unwrap().tick(); }
            _ => {}    
        }
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        match self.tabs.index {
            LOCAL_GUI_CODE => { self.local_player.draw(f, chunk, &mut self.media_player); }
            PANDORA_GUI_CODE => { self.pandora_player.draw(f, chunk, &mut self.media_player); }
            SPOTIFY_GUI_CODE => {}
            DIR_GUI_CODE => { self.dir_select.as_mut().unwrap().draw(f, chunk); }
            _ => {}    
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
    terminal.backend_mut().clear()?;
    loop {
        match events.next()? {
            Event::Input(input) => match input {
                Key::Right => app.tabs.next(),
                Key::Left => app.tabs.previous(),
                Key::Char('d') => {
                    if app.tabs.index == LOCAL_GUI_CODE {
                        app.tabs.index = DIR_GUI_CODE;
                    } else {
                        app.input(input);
                    }
                }
                Key::Char('q') => { 
                    if app.tabs.index != DIR_GUI_CODE { break; }
                    else { app.input(input); }
                }
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
        })?;
    }
    Ok(())
}