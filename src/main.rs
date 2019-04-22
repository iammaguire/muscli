extern crate rfmod;
mod util { pub mod event; }

use std::env;
use std::io;
use termion::raw::{ RawTerminal, IntoRawMode };
use termion::event::Key;
use tui::Terminal;
use tui::backend::TermionBackend;
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart };
use tui::layout::{ Layout, Constraint, Direction };
use tui::style::{ Color, Modifier, Style};
use id3::{ Tag };
use glob::glob;
use util::event::{ Event, Events };

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

fn main() -> Result<(), failure::Error> {
    let args: Vec<String> = env::args().collect();
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let path = &args[1];
    let events = Events::new();

    let mut default_playlist = build_playlist_from_directory(&path).unwrap();
    let mut selected_song: Option<usize> = None; // song currently selected (may not be playing) in selectable list
    let mut playing_song: Option<usize> = None; // song currently playing 
    let mut playing_song_handle: Option<rfmod::Sound> = None;
    let mut playing_channel: Option<rfmod::Channel> = None;
    let mut song_list: Vec<String> = Vec::new(); 
    let mut rebuild_song_list = false;

    let searcher_layout = vec![Constraint::Percentage(100)];
    let player_layout = vec![Constraint::Percentage(50), Constraint::Percentage(50)];

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

    for s in &default_playlist.songs { song_list.push(s.name.clone()); }
    terminal.hide_cursor()?;

    loop {
        // User input
        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    playing_song_handle.unwrap();
                    playing_channel.unwrap();
                    break;
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
                    };
                    rebuild_song_list = true;
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
                    };
                    rebuild_song_list = true;
                }
                Key::Char(' ') => {
                    if selected_song != None {
                        playing_song = selected_song;
                        playing_song_handle = match fmod.create_sound(&default_playlist.songs[playing_song.unwrap()].path, None, None) {
                            Ok(s) => Some(s),
                            Err(err) => panic!("Error code: {:?}", err)
                        };
                        playing_channel = match playing_song_handle.as_ref().unwrap().play() {
                            Ok(c) => Some(c),
                            Err(err) => panic!("Play: {:?}", err)
                        };
                    }
                }
                _ => {}
            },
            Event::Tick => {
                
            }
        }

        if rebuild_song_list && selected_song != None {
            song_list.clear();
            for (idx, s) in default_playlist.songs.iter().enumerate() { 
                if idx == selected_song.unwrap() { song_list.push(format!("> {}", &s.name).clone()); }
                else { song_list.push(s.name.clone()); } 
            }
            rebuild_song_list = false;
        }

        // Draw UI
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(match playing_song { Some(s) => player_layout.as_ref(), None => searcher_layout.as_ref() })
                .split(f.size());
            let song_list_style = Style::default().fg(Color::White);
            SelectableList::default()
                .block(Block::default().borders(Borders::ALL).title(&format!("Playlist: {}", default_playlist.name)))
                .items(&song_list)
                .select(playing_song)
                .style(song_list_style)
                .highlight_style(song_list_style.modifier(Modifier::BOLD))
                .render(&mut f, chunks[chunks.len() - 1]);
            if playing_song != None {
                let time_ms = playing_channel.as_ref().unwrap().get_position(rfmod::TIMEUNIT_MS).unwrap() as f32;
                let time_s = time_ms / 1000.0;
                let time_m = time_s / 60.0;
                let spectrum_data = playing_channel.as_ref().unwrap().get_wave_data(50, 0).unwrap();
                let mut spectrum_tuples: Vec<(&str, u64)> = Vec::new();
                for &s in spectrum_data.iter() { spectrum_tuples.push(("", (s.abs() * 100.0 + 1.0) as u64)); }

                let player_chunks = Layout::default()
                    .constraints([Constraint::Percentage(91), Constraint::Percentage(9)].as_ref())
                    .direction(Direction::Vertical)
                    .split(chunks[0]);
                BarChart::default()
                    .block(Block::default().title(&format!("{}", default_playlist.songs[playing_song.unwrap()].name)).borders(Borders::ALL))
                    .bar_width(1)
                    .bar_gap(1)
                    .style(Style::default().fg(Color::White))
                    .label_style(Style::default().fg(Color::White))
                    .data(&spectrum_tuples[..])
                    .max(100)
                    .render(&mut f, player_chunks[0]);
                Gauge::default()
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .percent((time_ms / playing_song_handle.as_ref().unwrap().get_length(rfmod::TIMEUNIT_MS).unwrap() as f32 * 100.0) as u16)
                    .label(&format!("{}{}:{}{}", if time_m < 10.0 { "0" } else { "" }, time_m as u32, if time_s < 10.0 { "0" } else { "" }, time_s as u32))
                    .render(&mut f, player_chunks[1]);
            }
        }).unwrap();
    }
    Ok(())
}
