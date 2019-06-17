const SUPPORTED_FORMATED: [&str; 15] = ["aiff", "asf", "asx", "dls", "flac", "fsb", "it", "m3u", "mp3", "midi", "mod", "ogg", "pls", "s3m", "wav"];

use std::fs::{ DirEntry, read_dir };
use easycurses::Input;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, Paragraph, Text, List };
use tui::style::{ Color, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction };
use tui::terminal::Frame;
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
    
    fn add_file_to_file_list(&mut self, file: DirEntry, dir: bool) {
        if let Some(file_name) = file.file_name().to_str() {
            if dir {
                if let Some(file_name) = file.file_name().to_str() {
                    let mut prefix = String::from("/");
                    prefix.push_str(file_name);
                    self.valid_files.push(prefix);
                } 
            } else {
                if let Some(os_str_ext) = file.path().as_path().extension() {
                    if let Some(ext) = os_str_ext.to_str() {
                        if SUPPORTED_FORMATED.contains(&ext) {
                            self.valid_files.push(String::from(file_name));
                        }
                    }
                }
            }
        }
    }
    
    fn rebuild_file_list(&mut self) {
        self.valid_files.clear();
        if let Ok(path) = read_dir(&self.input) { // requires full path, can't use ~/ for now
            for file in path {
                if let Ok(file) = file {
                    if let Ok(meta) = file.metadata() {
                        if meta.file_type().is_dir() {
                            self.add_file_to_file_list(file, true);
                        } else if meta.file_type().is_file() {
                            self.add_file_to_file_list(file, false);
                        }
                    }
                }
            }
        }
    }

    pub fn input(&mut self, key: Input, app: &mut App) {
        match key {
            Input::Character('\n') => {
                app.rebuild_local_with_dir(&self.input);
            }
           Input::Character('/') => {
                self.input.push('/');
                self.rebuild_file_list();
            }
            Input::Character(c) => {
                self.input.push(c);
            }
            Input::KeyBackspace => {
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
        let messages = self.valid_files.iter().enumerate().map(|(_i, m)| Text::raw(format!("{}", m)));
        List::new(messages)
            .block(Block::default().borders(Borders::ALL))
            .render(f, chunks[1]);
    }
}