use easycurses::Input;
use rfmod::Sys;
use id3::Tag;
use glob::glob;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, SelectableList };
use tui::style::{ Color, Modifier, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::terminal::Frame;
use super::player::Player;
use super::Config;
use super::MediaPlayer;
use super::player::{ Song, Playlist };

pub struct LocalPlayer {
    config: Config,
    playlist: Playlist,
    selected_song: Option<usize>,
    playing_song: Option<usize>,
    song_list: Vec<String>,
    rebuild_song_list: bool,
}

impl LocalPlayer {
    pub fn new(config: Config) -> LocalPlayer {
        let path = config.local_dir.clone();
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
}

impl Player for LocalPlayer {
    fn input(&mut self, key: Input, fmod: &Sys, media_player: &mut MediaPlayer) {
        match key {
            Input::Character('s') => {
                media_player.pause();
                self.playing_song = None;
            }
            Input::KeyDown => {
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
            Input::KeyUp => {
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
            Input::Character('x') => {
                if self.playing_song != None {
                    media_player.back();
                }
            }
            Input::Character('z') => {
                if self.playing_song != None {
                    media_player.forward();
                }                    
            }
            Input::Character(' ') => {
                if self.selected_song != None {
                    if self.selected_song != self.playing_song {
                        self.playing_song = self.selected_song;
                        media_player.play_from_uri(fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
                    } else {
                        media_player.toggle_pause();
                    }
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, fmod: &Sys, media_player: &mut MediaPlayer) {
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
            if media_player.almost_over() {
                self.playing_song = if self.playing_song.unwrap() >= self.song_list.len() - 1 { Some(0) } else { Some(self.playing_song.unwrap() + 1) };
                media_player.play_from_uri(fmod, &self.playlist.songs[self.playing_song.unwrap()].path);
            }
        }
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect, media_player: &mut MediaPlayer) {
        if let Some(idx) = self.playing_song {
            media_player.draw(f, chunk, self.playlist.name.as_str(), 
                                         self.song_list.clone(), 
                                         idx,
                                         String::from(""),
                                         String::from(""));
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