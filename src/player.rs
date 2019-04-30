use termion::event::Key;
use tui::terminal::Frame;
use tui::backend::Backend;
use tui::widgets::{ Widget, Block, Borders, SelectableList, Gauge, BarChart, Paragraph, Text };
use tui::style::{ Color, Modifier, Style};
use tui::layout::{ Rect, Layout, Constraint, Direction, Alignment };
use rfmod::Sys;

pub trait Player {
    fn input(&mut self, key: Key, fmod: &Sys);
    fn tick(&mut self, fmod: &Sys);
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect);
}

pub struct MediaUI {
    num_spectrum_bars: usize,
    spectrum_data_last: Vec<f32>
}

impl MediaUI {
    pub fn new() -> MediaUI {
          MediaUI {
              num_spectrum_bars: 70,
              spectrum_data_last: vec![0f32; 70]
          }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect, 
                            list_title: &str, 
                            list_member_titles: Vec<String>,
                            selected_idx: usize,
                            playing_song_handle: &rfmod::Sound,
                            playing_channel: &rfmod::Channel) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunk);
        let select_list_style = Style::default().fg(Color::White);
        SelectableList::default()
            .block(Block::default().borders(Borders::ALL).title(list_title))
            .items(&list_member_titles)
            .select(Some(selected_idx))
            .style(select_list_style)
            .highlight_style(select_list_style.modifier(Modifier::BOLD))
            .render(f, chunks[1]);
        
        let time_ms = playing_channel.get_position(rfmod::TIMEUNIT_MS).unwrap() as f32;
        let time_s = time_ms / 1000.0 % 60.0;
        let time_m = time_ms / 1000.0 / 60.0;
        let spectrum_data = &playing_channel.get_wave_data(self.num_spectrum_bars, 1).unwrap();
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
            .block(Block::default().title(&format!("{}{}", list_member_titles[selected_idx], if false { " PAUSED" } else { "" })).borders(Borders::ALL))
            .alignment(Alignment::Left)
            .render(f, player_chunks[1]);
        Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .percent((time_ms / playing_song_handle.get_length(rfmod::TIMEUNIT_MS).unwrap() as f32 * 100.0) as u16)
            .label(&format!("{}{}:{}{}", if time_m < 10.0 { "0" } else { "" }, time_m as u32, if time_s < 10.0 { "0" } else { "" }, time_s as u32))
            .render(f, player_chunks[2]);
    }

    fn input(&mut self, key: Key, fmod: &Sys) {

    }

    fn tick(&mut self, fmod: &Sys) {

    }
}