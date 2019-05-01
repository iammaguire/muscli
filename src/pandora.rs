use pandora_rs2::Pandora;
use pandora_rs2::stations::Station;
use pandora_rs2::playlist::Track;
use rfmod::Sys;
use std::cmp;
use std::io;
use std::fs::File;
use std::process::Command;
use termion::event::Key;
use tui::backend::Backend;
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text };
use tui::terminal::Frame;
use tui::style::{ Color, Modifier, Style};
use tempdir::TempDir;
use super::player::Player;
use super::{ Config, MediaUI, LocalPlayer };

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
    temp_dir: TempDir,
    playing_song_handle: Option<rfmod::Sound>,
    playing_channel: Option<rfmod::Channel>,
    playing_song_file: Option<File>,
    media_ui: MediaUI
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
            selected_station: None,
            stations_names: stations_names,
            viewing_stations: true,
            rebuild_station_list: false,
            current_playlist: None,
            current_playlist_titles: None,
            temp_dir: TempDir::new("muscli").expect("Couldn't create temp directory"),
            playing_song_handle: None,
            playing_channel: None,
            playing_song_file: None,
            media_ui: MediaUI::new()
        }
    }

    fn download_track(&self, track: &Track) -> Result<(File, String), failure::Error> {
        let target = track.track_audio.as_ref().expect("Couldn't unwrap track_audio").high_quality.audio_url.clone();
        let mut response = reqwest::get(&target)?;

        let (mut dest, fname) = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("tmp.mp3");

            let fname = self.temp_dir.path().join(fname);
            (File::create(fname.clone())?, fname)
        };
        io::copy(&mut response, &mut dest)?;

        // Hacky convert mp4 to mp3 until I properly implement an audio backend
        // ffmpeg -i audio.mp4 -vn -acodec libmp3lame -ac 2 -qscale:a 4 -ar 48000 audio.mp3
        let mut mp4_file = fname.to_str().unwrap().to_string();
        Command::new("/usr/bin/sh").args(&["-c", &format!("ffmpeg -i {} -vn -acodec libmp3lame -ac 2 -qscale:a 4 -ar 48000 {}.mp3", &mp4_file, &mp4_file)]).output().expect("Error executing ffmpeg command");
        mp4_file.push_str(".mp3");

        Ok((dest, mp4_file))
    }

    fn next_track(&mut self, fmod: &Sys) { // assumes a track is playing
        if let Some(mut idx) = self.selected_idx {
            let cur_len = self.current_playlist.as_ref().unwrap().len();

            if cur_len != 0 { // not first iter
                idx += 1;
                self.selected_idx = Some(idx);
            }
            if idx >= cur_len { // grab next playlist
                self.next_playlist();
            }
            
            let mut playlist = self.current_playlist.clone().unwrap();
            let (song_file, file_path) = self.download_track(&playlist[idx]).expect("Error while downloading track.");
            let (phandle, pchannel) = LocalPlayer::play_song(fmod, &file_path);
            self.playing_song_handle = Some(phandle);
            self.playing_channel = Some(pchannel);
            self.current_playlist = Some(playlist);       
        }
    }

    fn next_playlist(&mut self) {
        let mut playlist = self.current_playlist.clone().unwrap();
        let station_handle = self.handle.stations();
        if let Some(idx) = self.selected_station {
            if let Ok(mut new_playlist) = station_handle.playlist(&self.stations[idx]).list() {
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
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
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
                self.media_ui.draw(f, chunk, self.stations_names[self.selected_station.unwrap()].as_str(), 
                                             playlist_titles.to_vec(), 
                                             idx, 
                                             self.playing_song_handle.as_ref().unwrap(), 
                                             self.playing_channel.as_ref().unwrap());
            }
        }
    }

    fn input(&mut self, key: Key, fmod: &Sys) {
        let selection_list_length = match self.viewing_stations {
            true => self.stations_names.len(),
            false => self.current_playlist_titles.as_ref().unwrap().len()
        };

        match key {
            Key::Char(' ') => {
                if self.viewing_stations {
                    self.current_playlist = Some(Vec::new());
                    self.selected_station = self.selected_idx;
                    self.selected_idx = Some(3);
                    self.next_track(fmod);
                }
            }
            Key::Char('n') => {
                if !self.viewing_stations {
                    self.next_track(fmod);
                }
            }
            Key::Char('z') => {
                if let Some(channel) = &self.playing_channel {
                    channel.set_position(cmp::max(0, channel.get_position(rfmod::TIMEUNIT_MS).unwrap() as i32 - 10000) as usize, rfmod::TIMEUNIT_MS);
                }
            }
            Key::Char('x') => {
                if let Some(channel) = &self.playing_channel {
                    channel.set_position(channel.get_position(rfmod::TIMEUNIT_MS).unwrap() + 10000, rfmod::TIMEUNIT_MS);
                }                    
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

    fn tick(&mut self, fmod: &Sys) {
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
            match self.selected_idx {
                Some(selected) => { // if song 1 ms from being done play next track
                    if self.playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() as u32 >= self.playing_song_handle.as_ref().unwrap().get_length(rfmod::TIMEUNIT_MS).unwrap() - 1 {
                        self.next_track(fmod);
                    }
                }
                None => {
                    self.selected_idx = Some(0);
                    let (song_file, file_path) = self.download_track(&self.current_playlist.as_ref().expect("Couldn't unwrap current playlist")[0]).expect("Error while downloading track.");
                    self.playing_song_file = Some(song_file);
                    let (phandle, pchannel) = LocalPlayer::play_song(fmod, &file_path);
                    self.playing_song_handle = Some(phandle);
                    self.playing_channel = Some(pchannel);
                }
            }
        }
    }
}