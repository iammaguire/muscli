use pandora_rs2::Pandora;
use pandora_rs2::stations::Station;
use rfmod::Sys;
use std::io;
use termion::event::Key;
use tui::Terminal;
use tui::backend::Backend;
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text, Tabs };
use tui::terminal::Frame;
use tui::style::{ Color, Modifier, Style};
use super::player::Player;
use super::Config;

pub struct PandoraPlayer {
    config: Config,
    handle: Pandora,
    stations: Vec<Station>,
    stations_names: Vec<String>,
    selected_idx: Option<usize>,
    song_names: Vec<String>,
    viewing_stations: bool,
    rebuild_station_list: bool
}

impl PandoraPlayer {
    pub fn new(config: Config) -> PandoraPlayer {
        let handle = Pandora::new(&config.username, &config.password).expect("Couldn't initialize pandora.");
        let stations = handle.stations().list();
        let mut stations_names: Vec<String> = Vec::new();
        for s in stations.as_ref().unwrap().iter() { stations_names.push(s.station_name.clone()); }
        PandoraPlayer {
            handle: handle,
            config: config,
            stations: stations.unwrap(),
            selected_idx: None,
            song_names: Vec::new(),
            stations_names: stations_names,
            viewing_stations: true,
            rebuild_station_list: false
        }
    }
}

impl Player for PandoraPlayer {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(match self.viewing_stations { false => vec![Constraint::Percentage(50), Constraint::Percentage(50)], true => vec![Constraint::Percentage(100)] })
            .split(chunk);
        let song_list_style = Style::default().fg(Color::White);
        SelectableList::default()
            .block(Block::default().borders(Borders::ALL).title(&format!("Station List")))
            .items(&self.stations_names)
            .select(self.selected_idx)
            .style(song_list_style)
            .highlight_style(song_list_style.modifier(Modifier::BOLD))
            .render(f, chunks[chunks.len() - 1]);
    }

    fn input(&mut self, key: Key, fmod: &Sys) {
        let selection_list_length = match self.viewing_stations {
            true => self.stations_names.len(),
            false => self.song_names.len()
        };

        match key {
            Key::Down => {
                self.selected_idx = if let Some(selected) = self.selected_idx {
                    if selected >= selection_list_length - 1 {
                        Some(0)
                    } else {
                        Some(selected + 1)
                    }
                } else {
                    Some(0)
                };
                
                if self.viewing_stations { self.rebuild_station_list = true; }
            }
            Key::Up => {
                self.selected_idx = if let Some(selected) = self.selected_idx {
                    if selected > 0 {
                        Some(selected - 1)
                    } else {
                        Some(selection_list_length - 1)
                    }
                } else {
                    Some(0)
                };

                if self.viewing_stations { self.rebuild_station_list = true; }
            }
            _ => {}
        }
    }

    fn tick(&mut self, fmod: &Sys) {
        // draw > in list
        if self.rebuild_station_list && self.selected_idx != None {
            self.stations_names.clear();
            for (idx, s) in self.stations.iter().enumerate() { 
                if idx == self.selected_idx.unwrap() { self.stations_names.push(format!("> {}", &s.station_name.clone()).clone()); }
                else { self.stations_names.push(s.station_name.clone()); } 
            }
            self.rebuild_station_list = false;
        }
    }
}