use pandora_rs2::Pandora;
use pandora_rs2::stations::{ ToStationToken, Station };
use pandora_rs2::playlist::{ ToTrackToken, RateTrackRequest, Track };
use pandora_rs2::method::Method;
use rfmod::Sys;
use std::fs::{ File };
use termion::event::Key;
use tui::backend::Backend;
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::widgets::{ Widget, Block, Borders, SelectableList };
use tui::terminal::Frame;
use tui::style::{ Color, Style};
use super::player::Player;
use super::{ Config, MediaPlayer };

pub struct PandoraPlayer {
    config: Config,
    handle: Pandora,
    stations: Vec<Station>,
    stations_names: Vec<String>,
    selected_idx: Option<usize>,
    selected_station: Option<usize>,
    viewing_stations: bool,
    rebuild_station_list: bool,
    current_playlist: Option<Vec<Track>>,
    current_playlist_titles: Option<Vec<String>>,
}

impl PandoraPlayer {
    pub fn new(config: Config) -> PandoraPlayer {
        let handle = Pandora::new(&config.pandora_username, &config.pandora_password).expect("Couldn't initialize pandora.");
        let stations = handle.stations().list();
        let mut stations_names: Vec<String> = Vec::new();
        for s in stations.as_ref().unwrap().iter() { stations_names.push(s.station_name.clone()); }
        PandoraPlayer {
            handle: handle,
            config: config,
            stations: stations.unwrap(),
            selected_idx: None,
            selected_station: None,
            stations_names: stations_names,
            viewing_stations: true,
            rebuild_station_list: false,
            current_playlist: Some(Vec::new()),
            current_playlist_titles: Some(Vec::new()),
        }
    }

    fn next_track(&mut self, fmod: &Sys, media_player: &mut MediaPlayer) {
        if let Some(mut idx) = self.selected_idx {
            let cur_len = self.current_playlist.as_ref().unwrap().len();
            
            if cur_len != 0 { // not first iter
                idx += 1;
                self.selected_idx = Some(idx);
            }
            
            if idx >= cur_len || cur_len == 0 {
                self.next_playlist();
            }

            let url = &self.current_playlist.as_ref().expect("Couldn't unwrap current playlist")[idx].additional_audio_url.clone().unwrap();
            media_player.play_from_uri(fmod, &url);
        }
    }

    fn next_playlist(&mut self) {
        let mut playlist = self.current_playlist.clone().unwrap();
        let station_handle = self.handle.stations();
        if let Some(idx) = self.selected_station {
            let current_playlist_handle = Some(station_handle.playlist(&self.stations[idx]));
            if let Ok(new_playlist) = current_playlist_handle.as_ref().unwrap().list() {
                let mut track_names: Vec<String> = self.current_playlist_titles.clone().unwrap_or(Vec::new());
                for s in new_playlist.iter() {
                    if let Some(title) = &s.song_name {
                        track_names.push(title.clone());
                        playlist.push(s.clone());
                    }
                }
                self.current_playlist_titles = Some(track_names);
                self.viewing_stations = false;
                self.current_playlist = Some(playlist);
            }
        }
    }
}

impl Player for PandoraPlayer {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect, media_player: &mut MediaPlayer) {
        if self.viewing_stations {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(100)])
                .split(chunk);
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title(&format!("Station List")))
                .items(&self.stations_names)
                .select(self.selected_idx)
                .style(Style::default().fg(Color::White))
                .render(f, chunks[0]);
        } else if let Some(playlist_titles) = self.current_playlist_titles.as_ref() {
            if let Some(idx) = self.selected_idx {
                media_player.draw(f, chunk, &self.stations_names[self.selected_station.unwrap()][2..], 
                                             playlist_titles.to_vec(), 
                                             idx, 
                                             self.current_playlist.as_ref().unwrap()[idx].artist_name.clone().unwrap(),
                                             self.current_playlist.as_ref().unwrap()[idx].album_name.clone().unwrap());
            }
        }
    }

    fn input(&mut self, key: Key, fmod: &Sys, media_player: &mut MediaPlayer) {
        let selection_list_length = match self.viewing_stations {
            true => self.stations_names.len(),
            false => self.current_playlist_titles.as_ref().unwrap().len()
        };

        match key {
            Key::Char(' ') => {
                if self.viewing_stations {
                    self.selected_station = self.selected_idx;
                    self.selected_idx = Some(0);
                    self.next_track(fmod, media_player);
                } else {
                    media_player.toggle_pause();
                }
            }
            Key::Char('n') => {
                if !self.viewing_stations {
                    self.next_track(fmod, media_player);
                }
            }
            Key::Char('x') => {
                if !self.viewing_stations {
                    media_player.back();
                }
            }
            Key::Char('z') => {
                if !self.viewing_stations {
                    media_player.forward();
                }                    
            }
            Key::Ctrl('b') => {
                if !self.viewing_stations {
                    if let Some(playlist) = self.current_playlist.as_ref() {
                        let track = &playlist[self.selected_idx.unwrap()];
                        self.handle.request_noop(Method::StationAddFeedback, Some(serde_json::to_value(RateTrackRequest {
                                                                                        station_token: self.stations[self.selected_station.unwrap()].to_station_token(),
                                                                                        track_token: track.to_track_token().unwrap_or("".to_owned()),
                                                                                        is_positive: false,
                                                                                    }).unwrap()));
                        self.next_track(fmod, media_player);
                    }
                }
            }
            Key::Char('s') => {
                    self.current_playlist = Some(Vec::new());
                    self.current_playlist_titles = Some(Vec::new());
                    self.selected_idx = self.selected_station;
                    media_player.pause();
                    self.viewing_stations = true;
            }
            Key::Down => {
                if self.viewing_stations { 
                    self.selected_idx = if let Some(selected) = self.selected_idx {
                        if selected >= selection_list_length - 1 {
                            Some(0)
                        } else {
                            Some(selected + 1)
                        }
                    } else {
                        Some(0)
                    };
                    self.rebuild_station_list = true; 
                }
            }
            Key::Up => {
                if self.viewing_stations { 
                    self.selected_idx = if let Some(selected) = self.selected_idx {
                        if selected > 0 {
                            Some(selected - 1)
                        } else {
                            Some(selection_list_length - 1)
                        }
                    } else {
                        Some(0)
                    };
                    self.rebuild_station_list = true; 
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, fmod: &Sys, media_player: &mut MediaPlayer) {
        // draw > in list
        if self.rebuild_station_list && self.viewing_stations {
            self.stations_names.clear();
            for (idx, s) in self.stations.iter().enumerate() { 
                if idx == self.selected_idx.unwrap() { self.stations_names.push(format!("> {}", &s.station_name.clone()).clone()); }
                else { self.stations_names.push(s.station_name.clone()); } 
            }
            self.rebuild_station_list = false;
        }

        if !self.viewing_stations {
            if let Some(selected) = self.selected_idx {
                if media_player.almost_over() { 
                    self.next_track(fmod, media_player);
                }
                
                if *media_player.last_song_title.as_ref().unwrap() != self.current_playlist_titles.as_ref().unwrap()[selected] {
                    self.next_track(fmod, media_player); // nonworking attempt at context switch
                }
            }
        }
    }
}