use termion::event::Key;
use rfmod::Sys;
use id3::Tag;
use glob::glob;
use std::io;
use std::cmp;
use tui::Terminal;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text, Tabs };
use tui::style::{ Color, Modifier, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use tui::terminal::Frame;
use super::player::Player;
use super::Config;

struct Song {
    name: String,
    path: String,
    artist: String,
    album: String,
    length: u32,
}

struct Playlist {
    songs: Vec<Song>,
    name: String,
    length: u32
}

pub struct LocalPlayer<'a> {
    config: &'a Config,
    playlist: Playlist,
    selected_song: Option<usize>,
    playing_song: Option<usize>,
    song_list: Vec<String>,
    rebuild_song_list: bool,
    playing_song_handle: rfmod::Sound,
    playing_channel: rfmod::Channel,
    num_spectrum_bars: usize,
    spectrum_data_last: Vec<f32>,
    fmod: &'a Sys
}

impl<'a> LocalPlayer<'a> {
    pub fn new(config: &'a Config, fmod: &'a rfmod::Sys) -> LocalPlayer<'a> {
        let path = "/home/meet/Music/Logic/The_Incredible_True_Story/";
        let mut song_list = Vec::new();
        let default_playlist = LocalPlayer::build_playlist_from_directory(&path).unwrap();
        let (mut playing_song_handle, mut playing_channel) = LocalPlayer::play_song(fmod, &default_playlist.songs[0].path);
        for s in &default_playlist.songs { song_list.push(s.name.clone()); }
        playing_channel.set_paused(true);

        LocalPlayer {
            config: config,
            playlist: default_playlist,
            selected_song: None,
            playing_song: None,
            song_list: song_list,
            rebuild_song_list: false,
            playing_song_handle: playing_song_handle,
            playing_channel: playing_channel,
            num_spectrum_bars: 70,
            spectrum_data_last: vec![0f32; 70],
            fmod: fmod
        }
    }

    fn build_playlist_from_directory(path: &str) -> Result<Playlist, io::Error> {
        let mut glob_path = String::from(path);
        let mut songs: Vec<Song> = Vec::new();
        let mut total_length = 0;
        glob_path.push_str("/*.mp3");
        for entry in glob(&glob_path).expect("Failed to read glob pattern.") {
            match entry {
                Ok(path) => {
                    let tag = Tag::read_from_path(path.to_str().unwrap()).unwrap();
                    let song = Song { 
                        name: String::from(match tag.title() { Some(s) => s, None => "" }), 
                        path: String::from(match path.to_str() { Some(s) => s, None => "" }),
                        artist: String::from(match tag.artist() { Some(s) => s, None => "" }),
                        album: String::from(match tag.album() { Some(s) => s, None => "" }),
                        length: match tag.duration() { Some(i) => i, None => 0 },
                    };
                    total_length += song.length;
                    songs.push(song);
                },
                Err(e) => println!("{:?}", e),
            }
        }

        Ok(Playlist { name: songs[0].album.clone(), songs: songs, length: total_length })
    }

    fn play_song(fmod: &rfmod::Sys, path: &str) -> (rfmod::Sound, rfmod::Channel) {
        let playing_song_handle = match fmod.create_sound(path, None, None) {
            Ok(s) => s,
            Err(err) => panic!("Error code: {:?}", err)
        };
        let playing_channel = match playing_song_handle.play() {
            Ok(c) => c,
            Err(err) => panic!("Play: {:?}", err)
        };
        (playing_song_handle, playing_channel)
    }
}

impl<'a> Player for LocalPlayer<'a> {
    fn input(&mut self, key: Key) {
        match key {
            Key::Char('c') => {
                self.playing_channel.set_paused(true);
                self.playing_song = None;
            }
            Key::Down => {
                self.selected_song = if let Some(selected) = self.selected_song {
                    if selected >= self.song_list.len() - 1 {
                        Some(0)
                    } else {
                        Some(selected + 1)
                    }
                } else {
                    Some(0)
                };
                self.rebuild_song_list = true;
            }
            Key::Up => {
                self.selected_song = if let Some(selected) = self.selected_song {
                    if selected > 0 {
                        Some(selected - 1)
                    } else {
                        Some(self.song_list.len() - 1)
                    }
                } else {
                    Some(0)
                };
                self.rebuild_song_list = true;
            }
            Key::Char('a') => {
                if self.playing_song != None {
                    self.playing_channel.set_position(cmp::max(0, self.playing_channel.get_position(rfmod::TIMEUNIT_MS).unwrap() as i32 - 10000) as usize, rfmod::TIMEUNIT_MS);
                }
            }
            Key::Char('s') => {
                if self.playing_song != None {
                    self.playing_channel.set_position(self.playing_channel.get_position(rfmod::TIMEUNIT_MS).unwrap() + 10000, rfmod::TIMEUNIT_MS);
                }                    
            }
            Key::Char(' ') => {
                if self.selected_song != None {
                    if self.selected_song != self.playing_song {
                        self.playing_song = self.selected_song;
                        let (phandle, pchannel) = LocalPlayer::play_song(self.fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
                        self.playing_song_handle = phandle;
                        self.playing_channel = pchannel;
                    } else {
                        self.playing_channel.set_paused(!self.playing_channel.get_paused().unwrap());
                    }
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        // draw > in list
        if self.rebuild_song_list && self.selected_song != None {
            self.song_list.clear();
            for (idx, s) in self.playlist.songs.iter().enumerate() { 
                if idx == self.selected_song.unwrap() { self.song_list.push(format!("> {}", &s.name).clone()); }
                else { self.song_list.push(s.name.clone()); } 
            }
            self.rebuild_song_list = false;
        }

        // go to next song
        if self.playing_song != None {
            if self.playing_channel.get_position(rfmod::TIMEUNIT_MS).unwrap() as u32 >= self.playing_song_handle.get_length(rfmod::TIMEUNIT_MS).unwrap() - 1 {
                self.playing_song = if self.playing_song.unwrap() >= self.song_list.len() - 1 { Some(0) } else { Some(self.playing_song.unwrap() + 1) };
                let (phandle, pchannel) = LocalPlayer::play_song(self.fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
                self.playing_song_handle = phandle;
                self.playing_channel = pchannel;
            }
        }
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(match self.playing_song { Some(s) => vec![Constraint::Percentage(50), Constraint::Percentage(50)], None => vec![Constraint::Percentage(100)] })
            .split(chunk);
        let song_list_style = Style::default().fg(Color::White);
        SelectableList::default()
            .block(Block::default().borders(Borders::ALL).title(&format!("Playlist: {}", self.playlist.name)))
            .items(&self.song_list)
            .select(self.playing_song)
            .style(song_list_style)
            .highlight_style(song_list_style.modifier(Modifier::BOLD))
            .render(f, chunks[chunks.len() - 1]);
        if self.playing_song != None {
            let time_ms = self.playing_channel.get_position(rfmod::TIMEUNIT_MS).unwrap() as f32;
            let time_s = time_ms / 1000.0 % 60.0;
            let time_m = time_ms / 1000.0 / 60.0;
            let spectrum_data = &self.playing_channel.get_wave_data(self.num_spectrum_bars, 1).unwrap();
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
                .block(Block::default().title(&format!("{}{}", self.playlist.songs[self.playing_song.unwrap()].name, if self.playing_channel.get_paused().unwrap() { " PAUSED" } else { "" })).borders(Borders::ALL))
                .alignment(Alignment::Left)
                .render(f, player_chunks[1]);
            Gauge::default()
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .percent((time_ms / self.playing_song_handle.get_length(rfmod::TIMEUNIT_MS).unwrap() as f32 * 100.0) as u16)
                .label(&format!("{}{}:{}{}", if time_m < 10.0 { "0" } else { "" }, time_m as u32, if time_s < 10.0 { "0" } else { "" }, time_s as u32))
                .render(f, player_chunks[2]);
        }
    }
}