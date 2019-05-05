const SUPPORTED_FORMATED: [&str; 15] = ["aiff", "asf", "asx", "dls", "flac", "fsb", "it", "m3u", "mp3", "midi", "mod", "ogg", "pls", "s3m", "wav"];

use std::fs::read_dir;
use std::path::Path;
use termion::event::Key;
use termion::cursor::Goto;
use rfmod::Sys;
use id3::Tag;
use glob::glob;
use tui::Terminal;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, Paragraph, Text, List };
use tui::style::{ Color, Modifier, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::terminal::Frame;
use super::Config;
use super::MediaPlayer;
use super::App;

#[derive(Clone)]
pub struct DirSelect {
    input: String,
    valid_files: Vec<String>
}

impl DirSelect {
    pub fn new() -> DirSelect {
        DirSelect {
            input: String::new(),
            valid_files: Vec::new()
        }
    }
    
    fn rebuild_file_list(&mut self) {
        self.valid_files.clear();
        if let Ok(path) = read_dir(&self.input) { // requires full path, can't use ~/ for now
            for file in path {
                if let Ok(file) = file {
                    if let Ok(meta) = file.metadata() {
                        if meta.file_type().is_file() {
                            if let Some(os_str_ext) = file.path().as_path().extension() {
                                if let Some(ext) = os_str_ext.to_str() {
                                    if SUPPORTED_FORMATED.contains(&ext) {
                                        if let Some(file_name) = file.file_name().to_str() {
                                            self.valid_files.push(String::from(file_name)); // what even is an error anyways
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn input(&mut self, key: Key, app: &mut App) {
        match key {
            Key::Char('\n') => {
                app.rebuild_local_with_dir(&self.input);
            }
            Key::Char(c) => {
                self.input.push(c);
                self.rebuild_file_list();
            }
            Key::Backspace => {
                self.input.pop();
                self.rebuild_file_list();
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(chunk);
        Paragraph::new([Text::raw(&self.input)].iter())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .render(f, chunks[0]);
        let messages = self.valid_files.iter().enumerate().map(|(i, m)| Text::raw(format!("{}", m)));
        List::new(messages)
            .block(Block::default().borders(Borders::ALL))
            .render(f, chunks[1]);
    }
}