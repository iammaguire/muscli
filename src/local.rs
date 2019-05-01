use termion::event::Key;
use rfmod::Sys;
use id3::Tag;
use glob::glob;
use std::cmp;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, SelectableList };
use tui::style::{ Color, Modifier, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::terminal::Frame;
use super::player::Player;
use super::Config;
use super::MediaUI;

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

pub struct LocalPlayer {
    config: Config,
    playlist: Playlist,
    selected_song: Option<usize>,
    playing_song: Option<usize>,
    song_list: Vec<String>,
    rebuild_song_list: bool,
    playing_song_handle: Option<rfmod::Sound>,
    playing_channel: Option<rfmod::Channel>,
    media_ui: MediaUI
}

impl LocalPlayer {
    pub fn new(config: Config) -> LocalPlayer {
        let path = "/home/meet/Music/Logic/The_Incredible_True_Story/";
        let mut song_list = Vec::new();
        let default_playlist = LocalPlayer::build_playlist_from_directory(&path).unwrap();
        for s in &default_playlist.songs { song_list.push(s.name.clone()); }

        LocalPlayer {
            config: config,
            playlist: default_playlist,
            selected_song: None,
            playing_song: None,
            song_list: song_list,
            rebuild_song_list: false,
            playing_song_handle: None,
            playing_channel: None,
            media_ui: MediaUI::new()
        }
    }

    fn build_playlist_from_directory(path: &str) -> Result<Playlist, failure::Error> {
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

    pub fn play_song(fmod: &rfmod::Sys, path: &str) -> (rfmod::Sound, rfmod::Channel) {
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

impl Player for LocalPlayer {
    fn input(&mut self, key: Key, fmod: &Sys) {
        match key {
            Key::Char('s') => {
                self.playing_channel.as_ref().unwrap().set_paused(true);
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
            Key::Char('z') => {
                if self.playing_song != None {
                    self.playing_channel.as_ref().unwrap().set_position(cmp::max(0, self.playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() as i32 - 10000) as usize, rfmod::TIMEUNIT_MS);
                }
            }
            Key::Char('x') => {
                if self.playing_song != None {
                    self.playing_channel.as_ref().unwrap().set_position(self.playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() + 10000, rfmod::TIMEUNIT_MS);
                }                    
            }
            Key::Char(' ') => {
                if self.selected_song != None {
                    if self.selected_song != self.playing_song {
                        self.playing_song = self.selected_song;
                        let (phandle, pchannel) = LocalPlayer::play_song(fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
                        self.playing_song_handle = Some(phandle);
                        self.playing_channel = Some(pchannel);
                    } else {
                        self.playing_channel.as_ref().unwrap().set_paused(!self.playing_channel.as_ref().unwrap().get_paused().unwrap());
                    }
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, fmod: &Sys) {
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
            if self.playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() as u32 >= self.playing_song_handle.as_ref().unwrap().get_length(rfmod::TIMEUNIT_MS).unwrap() - 1 {
                self.playing_song = if self.playing_song.unwrap() >= self.song_list.len() - 1 { Some(0) } else { Some(self.playing_song.unwrap() + 1) };
                let (phandle, pchannel) = LocalPlayer::play_song(fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
                self.playing_song_handle = Some(phandle);
                self.playing_channel = Some(pchannel);
            }
        }
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if let Some(idx) = self.playing_song {
            self.media_ui.draw(f, chunk, self.playlist.name.as_str(), 
                                         self.song_list.clone(), 
                                         idx, 
                                         self.playing_song_handle.as_ref().unwrap(), 
                                         self.playing_channel.as_ref().unwrap());
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(100)])
                .split(chunk);
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title(&format!("Playlist: {}", self.playlist.name)))
                .items(&self.song_list)
                .select(self.playing_song)
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::White).modifier(Modifier::BOLD))
                .render(f, chunks[chunks.len() - 1]);
        }
    }
}