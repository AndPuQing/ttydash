use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use super::Component;
use crate::{
    action::Action,
    cli::{self, Cli},
    config,
    data_pipeline::{DashState, DataPipeline, Extractor},
};
use color_eyre::Result;

use ratatui::{prelude::*, widgets::*};

use symbols::bar;
use tokio::{sync::mpsc::UnboundedSender, task};

#[derive(Debug, Default, Clone)]
pub struct Dash {
    bar_set: bar::Set<'static>,
    group: bool,
    layout: cli::Layout,

    state: Arc<RwLock<Vec<DashState>>>,
    titles: Option<Vec<String>>,
    threshold_low: Option<f64>,
    threshold_low_color: Color,
    threshold_high: Option<f64>,
    threshold_high_color: Color,

    command_tx: Option<UnboundedSender<Action>>,
    stop_signal: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

fn parse_color(s: &str) -> Color {
    if s.starts_with('#') {
        let s = if s.len() == 7 {
            s.to_string()
        } else if s.len() == 4 {
            format!(
                "#{}{}{}{}{}{}",
                &s[1..2],
                &s[1..2],
                &s[2..3],
                &s[2..3],
                &s[3..4],
                &s[3..4]
            )
        } else {
            s.to_string()
        };
        if let Ok(hex) = u32::from_str_radix(&s[1..], 16) {
            let r = ((hex >> 16) & 0xFF) as u8;
            let g = ((hex >> 8) & 0xFF) as u8;
            let b = (hex & 0xFF) as u8;
            return Color::Rgb(r, g, b);
        }
    }
    s.parse::<Color>().unwrap_or(Color::White)
}

impl Dash {
    pub fn new(args: Cli, is_paused: Arc<AtomicBool>) -> Self {
        let bar_set = bar::Set {
            full: "⣿",
            seven_eighths: "⣾",
            three_quarters: "⣶",
            five_eighths: "⣴",
            half: "⣤",
            three_eighths: "⣠",
            one_quarter: "⣀",
            one_eighth: "⢀",
            empty: " ",
        };
        let stop_signal = Arc::new(AtomicBool::new(false));
        let state = Arc::new(RwLock::new(vec![DashState::default()]));
        let mut extractors = Vec::new();
        let predefined_regexes = config::get_regexes().unwrap_or_default();
        if let Some(regex_keys) = args.regex {
            for key in regex_keys {
                if let Some(re_str) = predefined_regexes.get(&key) {
                    if let Ok(re) = regex::Regex::new(re_str) {
                        extractors.push(Extractor::Regex(re));
                    }
                } else if let Ok(re) = regex::Regex::new(&key) {
                    extractors.push(Extractor::Regex(re));
                }
            }
        }
        if let Some(units) = args.units {
            for unit in units {
                let pattern = format!(r"(?i)\b([+-]?\d+(?:\.\d+)?)\s*{}\b", regex::escape(&unit));
                if let Ok(regex) = regex::Regex::new(&pattern) {
                    extractors.push(Extractor::Unit { unit, regex });
                }
            }
        }
        if let Some(indices) = args.indices {
            for index in indices {
                extractors.push(Extractor::Index(index.saturating_sub(1)));
            }
        }

        let data_pipeline = DataPipeline::new(
            state.clone(),
            extractors,
            args.update_frequency,
            stop_signal.clone(),
            is_paused.clone(),
        );
        task::spawn(data_pipeline.run());

        Self {
            titles: args.titles,
            state,
            group: args.group.unwrap_or(false),
            command_tx: None,
            bar_set,
            layout: args.layout.unwrap_or_default(),
            stop_signal,
            is_paused,
            threshold_low: args.threshold_low,
            threshold_low_color: parse_color(&args.threshold_low_color),
            threshold_high: args.threshold_high,
            threshold_high_color: parse_color(&args.threshold_high_color),
        }
    }
}

impl Drop for Dash {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

fn generate_time_markers(window_size: u16, state_len: usize) -> Vec<Span<'static>> {
    if window_size <= 5 {
        return Vec::new();
    }

    let time_labels = (1..)
        .map(|i| i * 30)
        .take_while(|&t| t <= window_size.saturating_sub(5))
        .collect::<Vec<_>>();
    time_labels
        .iter()
        .scan(0, |last_label_len, &time| {
            let pos = window_size - time - 1;
            if pos < window_size {
                let time_marker = format!("{time}s");
                let time_marker_len = time_marker.len() + 1;
                let spacing = "─".repeat(
                    30usize
                        .saturating_mul(state_len)
                        .saturating_sub(*last_label_len),
                );
                *last_label_len = time_marker_len;
                Some(vec![
                    Span::raw(spacing),
                    Span::raw("├"),
                    Span::styled(time_marker, Style::default().gray()),
                ])
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

impl Dash {
    fn draw_grouped_chart(&mut self, frame: &mut Frame, area: &Rect) -> Result<()> {
        let state = self.state.read().unwrap();
        if state.is_empty() {
            return Ok(());
        }

        let window_size = area.width.saturating_sub(1) / state.len() as u16;

        let span_vec = generate_time_markers(window_size, state.len());

        let mut chart = BarChart::default()
            .bar_set(self.bar_set.clone())
            .bar_gap(0)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .title(
                        Line::from(if self.is_paused.load(Ordering::Relaxed) {
                            "Group Chart ⏸"
                        } else {
                            "Group Chart"
                        })
                        .right_aligned(),
                    ) // Add chart title
                    .title_bottom(Line::from(span_vec)) // Add time markers
                    .title_alignment(Alignment::Right)
                    .borders(Borders::ALL),
            )
            .bar_width(1)
            .group_gap(0);

        // Define a color map to style the bars
        let color_map = [
            Color::Green,
            Color::Red,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
            Color::White,
        ];

        for offset in (1..=window_size).rev() {
            let bars = (0..state.len())
                .map(|n| {
                    let value = state[n].value_from_end(offset.into()).unwrap_or(0.0);
                    let mut bar = Bar::default()
                        .value(value as u64)
                        .text_value("".to_owned())
                        .style(Style::default().fg(color_map[n % color_map.len()]));
                    if let Some(threshold_low) = self.threshold_low {
                        if value < threshold_low {
                            bar = bar.style(Style::default().fg(self.threshold_low_color));
                        }
                    }
                    if let Some(threshold_high) = self.threshold_high {
                        if value > threshold_high {
                            bar = bar.style(Style::default().fg(self.threshold_high_color));
                        }
                    }
                    bar
                })
                .collect::<Vec<_>>();
            chart = chart.data(BarGroup::default().bars(&bars));
        }

        frame.render_widget(chart, *area);

        let max_value = state.iter().map(|s| s.max_value).fold(0.0, f64::max);

        let [top, _] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(*area);
        let y_message = format!("{:.0}{}", max_value, state[0].unit);
        let y_span = Span::styled(y_message, Style::new().dim().fg(Color::DarkGray));
        let y_paragraph = Paragraph::new(y_span)
            .left_aligned()
            .block(Block::default().padding(Padding {
                left: 2,
                right: 0,
                top: 1,
                bottom: 0,
            }));
        frame.render_widget(y_paragraph, top);

        Ok(())
    }

    fn draw_chart(&mut self, frame: &mut Frame, area: &Rect, i: usize) -> Result<()> {
        let mut title = self
            .titles
            .as_ref()
            .and_then(|titles| titles.get(i))
            .unwrap_or(&format!("Chart {}", i + 1))
            .to_string();
        if self.is_paused.load(Ordering::Relaxed) {
            title.push_str(" ⏸");
        }
        let state = self.state.read().unwrap();
        let state = &state[i];
        let width = area.width.saturating_sub(1) as usize;
        let bars = std::iter::repeat_n(0.0, width.saturating_sub(state.length))
            .chain(state.recent_values(width))
            .map(|value| {
                let mut bar = Bar::default().value(value as u64).text_value("".to_owned());
                if let Some(threshold_low) = self.threshold_low {
                    if value < threshold_low {
                        bar = bar.style(Style::default().fg(self.threshold_low_color));
                    }
                }
                if let Some(threshold_high) = self.threshold_high {
                    if value > threshold_high {
                        bar = bar.style(Style::default().fg(self.threshold_high_color));
                    }
                }
                bar
            })
            .collect::<Vec<_>>();

        let span_vec = generate_time_markers(width as u16, 1);
        let chart = BarChart::default()
            .data(BarGroup::default().bars(&bars))
            .bar_set(self.bar_set.clone())
            .bar_gap(0)
            .bar_style(Style::default().fg(Color::Green))
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .title(Line::from(title).right_aligned())
                    .title_bottom(Line::from(span_vec))
                    .title_alignment(Alignment::Right)
                    .borders(Borders::ALL),
            )
            .bar_width(1);
        frame.render_widget(chart, *area);

        let [top, _] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(*area);

        let message = format!(
            "Avg: {:.2} {} Min: {:.2} {} Max: {:.2} {}",
            state.average, state.unit, state.min_value, state.unit, state.max_value, state.unit
        );
        let span = Span::styled(message, Style::new().dim());
        let paragraph = Paragraph::new(span)
            .left_aligned()
            .block(Block::default().padding(Padding::horizontal(2)));
        frame.render_widget(paragraph, top);

        let [top, _] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(*area);
        let max_value = state.max_value;
        let y_message = format!("{:.0}{}", max_value, state.unit);
        let y_span = Span::styled(y_message, Style::new().dim().fg(Color::DarkGray));
        let y_paragraph = Paragraph::new(y_span)
            .left_aligned()
            .block(Block::default().padding(Padding {
                left: 2,
                right: 0,
                top: 1,
                bottom: 0,
            }));
        frame.render_widget(y_paragraph, top);

        Ok(())
    }
}

fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    for i in 2..=((n as f64).sqrt() as usize) {
        if n.is_multiple_of(i) {
            return false;
        }
    }
    true
}

impl Component for Dash {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.group {
            let state = self.state.read().unwrap();
            let num_chart_states = state.len();
            // split the area
            let chunks = match self.layout {
                cli::Layout::Vertical => {
                    Layout::vertical(vec![Constraint::Percentage(100); num_chart_states])
                        .split(area)
                        .iter()
                        .copied()
                        .collect::<Vec<_>>()
                }
                cli::Layout::Horizontal => {
                    Layout::horizontal(vec![Constraint::Percentage(100); num_chart_states])
                        .split(area)
                        .iter()
                        .copied()
                        .collect::<Vec<_>>()
                }
                cli::Layout::Auto => {
                    if is_prime(num_chart_states) {
                        // grid + 1
                        let (rows, cols) = match num_chart_states - 1 {
                            1 => (1, 1),
                            2 => (1, 2),
                            _ => {
                                let rows = (2..=num_chart_states - 1)
                                    .rev()
                                    .find(|&i| num_chart_states.is_multiple_of(i))
                                    .unwrap_or(1);
                                let cols = num_chart_states / rows;
                                (rows, cols)
                            }
                        };
                        let row_constraints =
                            vec![Constraint::Percentage((100 / rows + 1) as u16); rows + 1];
                        let row_chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(row_constraints)
                            .split(area);
                        let mut chunks = vec![];
                        for row_chunk in row_chunks[1..].iter() {
                            let col_constraints =
                                vec![Constraint::Percentage((100 / cols) as u16); cols];
                            let col_chunks = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints(col_constraints)
                                .split(*row_chunk);
                            let col_chunks_vec = col_chunks.iter().copied().collect::<Vec<_>>();
                            chunks.extend(col_chunks_vec);
                        }
                        chunks.insert(0, row_chunks[0]);
                        chunks
                    } else {
                        let (rows, cols) = match num_chart_states {
                            1 => (1, 1),
                            2 => (1, 2),
                            _ => {
                                let rows = (2..=num_chart_states - 1)
                                    .rev()
                                    .find(|&i| num_chart_states.is_multiple_of(i))
                                    .unwrap_or(1);
                                let cols = num_chart_states / rows;
                                (rows, cols)
                            }
                        };
                        let row_constraints =
                            vec![Constraint::Percentage((100 / rows) as u16); rows];
                        let row_chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(row_constraints)
                            .split(area);
                        let mut chunks = vec![];
                        for row_chunk in row_chunks.iter() {
                            let col_constraints =
                                vec![Constraint::Percentage((100 / cols) as u16); cols];
                            let col_chunks = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints(col_constraints)
                                .split(*row_chunk);
                            let col_chunks_vec = col_chunks.iter().copied().collect::<Vec<_>>();
                            chunks.extend(col_chunks_vec);
                        }
                        chunks
                    }
                }
            };
            // release the lock
            drop(state);
            for (i, chunk) in chunks.iter().enumerate() {
                self.draw_chart(frame, chunk, i)?;
            }
        } else {
            self.draw_grouped_chart(frame, &area)?;
        }
        Ok(())
    }
}
