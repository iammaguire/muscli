use pandora_rs2::Pandora;
use pandora_rs2::stations::Station;
use pandora_rs2::playlist::Track;
use rfmod::Sys;
use std::option;
use std::io;
use std::fs::File;
use std::process::Command;
use termion::event::Key;
use tui::Terminal;
use tui::backend::Backend;
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text, Tabs };
use tui::terminal::Frame;
use tui::style::{ Color, Modifier, Style};
use tempdir::TempDir;
use super::player::Player;
use super::LocalPlayer;
use super::Config;

use std::{thread, time};

pub struct PandoraPlayer {
    config: Config,
    handle: Pandora,
    stations: Vec<Station>,
    stations_names: Vec<String>,
    selected_idx: Option<usize>,
    song_names: Vec<String>,
    viewing_stations: bool,
    rebuild_station_list: bool,
    current_playlist: Option<Vec<Track>>,
    current_playlist_titles: Option<Vec<String>>,
    temp_dir: TempDir,
    playing_song_handle: Option<rfmod::Sound>,
    playing_channel: Option<rfmod::Channel>,
    playing_song_file: Option<File>,
    num_spectrum_bars: usize,
    spectrum_data_last: Vec<f32>
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
            rebuild_station_list: false,
            current_playlist: None,
            current_playlist_titles: None,
            temp_dir: TempDir::new("muscli").expect("Couldn't create temp directory"),
            playing_song_handle: None,
            playing_channel: None,
            playing_song_file: None,
            num_spectrum_bars: 70,
            spectrum_data_last: vec![0f32; 70]
        }
    }

    fn download_track(&self, track: &Track) -> Result<(File, String), failure::Error> {
        let target = track.track_audio.as_ref().expect("Couldn't unwrap track_audio").high_quality.audio_url.clone();
        let mut response = reqwest::get(&target)?;

        let (mut dest, mut fname) = {
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
}

impl Player for PandoraPlayer {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(match self.viewing_stations { false => vec![Constraint::Percentage(50), Constraint::Percentage(50)], true => vec![Constraint::Percentage(100)] })
            .split(chunk);
        let select_list_style = Style::default().fg(Color::White);
        if self.viewing_stations {
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title(&format!("Station List")))
                .items(&self.stations_names)
                .select(self.selected_idx)
                .style(select_list_style)
                .render(f, chunks[0]);
        } else if let Some(playlist_titles) = self.current_playlist_titles.as_ref() {
            if self.selected_idx != None {
                SelectableList::default()
                    .block(Block::default().borders(Borders::ALL).title(&format!("Track List")))
                    .items(playlist_titles)
                    .select(self.selected_idx)
                    .style(select_list_style)
                    .highlight_style(select_list_style.modifier(Modifier::BOLD))
                    .render(f, chunks[1]);
                
                let time_ms = self.playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() as f32;
                let time_s = time_ms / 1000.0 % 60.0;
                let time_m = time_ms / 1000.0 / 60.0;
                let spectrum_data = &self.playing_channel.as_ref().unwrap().get_wave_data(self.num_spectrum_bars, 1).unwrap();
                let mut spectrum_tuples: Vec<(&str, u64)> = Vec::new();
                for (idx, &s) in spectrum_data.iter().enumerate() { 
                    let value = (self.spectrum_data_last[idx].abs() + s.abs()) / 2.0 * 100.0 + 2.0;
                    spectrum_tuples.push(("", value as u64)); 
                    self.spectrum_data_last[idx] = s;
                }

                let info_text = [
                    Text::raw("Artist: \nDate: \nLength: \n# plays: "),
                ];
                
                let player_chunks = Layout::default()
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(50), Constraint::Percentage(10)].as_ref())
                    .direction(Direction::Vertical)
                    .split(chunks[0]);
                BarChart::default()
                    .block(Block::default().borders(Borders::ALL))
                    .bar_width(1)
                    .bar_gap(1)
                    .style(Style::default().fg(Color::White))
                    .data(&spectrum_tuples)
                    .max(100)
                    .render(f, player_chunks[0]);
                Paragraph::new(info_text.iter())
                    .block(Block::default().title(&format!("{}{}", playlist_titles[self.selected_idx.unwrap()], if false { " PAUSED" } else { "" })).borders(Borders::ALL))
                    .alignment(Alignment::Left)
                    .render(f, player_chunks[1]);
                Gauge::default()
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .percent((time_ms / self.playing_song_handle.as_ref().unwrap().get_length(rfmod::TIMEUNIT_MS).unwrap() as f32 * 100.0) as u16)
                    .label(&format!("{}{}:{}{}", if time_m < 10.0 { "0" } else { "" }, time_m as u32, if time_s < 10.0 { "0" } else { "" }, time_s as u32))
                    .render(f, player_chunks[2]);
            }
        }
    }

    fn input(&mut self, key: Key, fmod: &Sys) {
        let selection_list_length = match self.viewing_stations {
            true => self.stations_names.len(),
            false => self.song_names.len()
        };

        match key {
            Key::Char(' ') => {
                if self.viewing_stations {
                    let selected_station = &self.stations_names[self.selected_idx.unwrap()];
                    let station_handle = self.handle.stations();
                    if let Ok(s) = station_handle.playlist(&self.stations[self.selected_idx.unwrap()]).list() {
                        self.current_playlist = Some(s);
                        let mut track_names: Vec<String> = Vec::new();
                        for s in self.current_playlist.clone().unwrap().iter() { 
                            track_names.push(match s.song_name.as_ref() {
                                Some(s) => s.clone(),
                                None => String::from("Null song")
                            }); 
                        }
                        self.current_playlist_titles = Some(track_names);
                        self.selected_idx = None;
                        self.viewing_stations = false;
                    }
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
                Some(selected) => {
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