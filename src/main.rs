mod util { pub mod event; }

use std::fs;
use std::env;
use std::io;
use termion::raw::{ RawTerminal, IntoRawMode };
use termion::event::Key;
use tui::Terminal;
use tui::backend::TermionBackend;
use tui::widgets::{ Widget, Block, Borders, SelectableList };
use tui::layout::{ Layout, Constraint, Direction };
use tui::style::{Color, Modifier, Style};
use id3::{Tag, Version};
use glob::glob;
use util::event::{Event, Events};

struct Song {
    name: String,
    path: String,
    artist: String,
    album: String,
    length: u32
}

struct Playlist {
    songs: Vec<Song>,
    name: String,
    length: u32
}

fn build_playlist_from_directory(path: &str) -> Result<Playlist, io::Error> {
    let mut glob_path = String::from(path);
    let mut songs: Vec<Song> = Vec::new();
    let mut total_length = 0;
    glob_path.push_str("/*.mp3");
    for entry in glob(&glob_path).expect("Failed to read glob pattern.") {
        match entry {
            Ok(path) => {
                let mut tag = Tag::read_from_path(path.to_str().unwrap()).unwrap();
                let song = Song { 
                    name: String::from(match tag.title() { Some(s) => s, None => "" }), 
                    path: String::from(match path.to_str() { Some(s) => s, None => "" }),
                    artist: String::from(match tag.artist() { Some(s) => s, None => "" }),
                    album: String::from(match tag.album() { Some(s) => s, None => "" }),
                    length: match tag.duration() { Some(i) => i, None => 0 }
                };
                total_length += song.length;
                songs.push(song);
            },
            Err(e) => println!("{:?}", e),
        }
    }

    Ok(Playlist { name: songs[0].album.clone(), songs: songs, length: total_length })
}

fn main() -> Result<(), failure::Error> {
    let args: Vec<String> = env::args().collect();
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut path = &args[1];
    let events = Events::new();

    let default_playlist = build_playlist_from_directory(&path).unwrap();
    let mut selected_song: Option<usize> = None;
    let mut song_list = Vec::new();
    for s in &default_playlist.songs { song_list.push(&s.name); }
    terminal.hide_cursor()?;

    loop {
        // Draw UI
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title(&format!("Playlist: {}", default_playlist.name)))
                .items(&song_list)
                .select(selected_song)
                .style(Style::default())
                .highlight_symbol(">")
                .render(&mut f, chunks[1]);
            Block::default()
                .title(&format!("Artist: {}", default_playlist.songs[0].artist))
                .borders(Borders::ALL)
                .render(&mut f, chunks[0]);
        });

        // User input
        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    break;
                }
                Key::Left => {
                    selected_song = None;
                }
                Key::Down => {
                    selected_song = if let Some(selected) = selected_song {
                        if selected >= song_list.len() - 1 {
                            Some(0)
                        } else {
                            Some(selected + 1)
                        }
                    } else {
                        Some(0)
                    }
                }
                Key::Up => {
                    selected_song = if let Some(selected) = selected_song {
                        if selected > 0 {
                            Some(selected - 1)
                        } else {
                            Some(song_list.len() - 1)
                        }
                    } else {
                        Some(0)
                    }
                }
                _ => {}
            },
            Event::Tick => {
                
            }
        }
    }
    Ok(())
}
