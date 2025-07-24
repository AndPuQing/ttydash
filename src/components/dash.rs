use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use super::Component;
use crate::{
    action::Action,
    cli::{self, Cli},
    data_pipeline::{DashState, DataPipeline},
};
use color_eyre::Result;

use ratatui::{prelude::*, widgets::*};

use symbols::bar;
use tokio::{sync::mpsc::UnboundedSender, task};

#[derive(Debug, Default, Clone)]
pub struct Dash {
    bar_set: bar::Set,
    group: bool,
    layout: cli::Layout,

    state: Arc<RwLock<Vec<DashState>>>,
    titles: Option<Vec<String>>,

    command_tx: Option<UnboundedSender<Action>>,
    stop_signal: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
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
        let data_pipeline = DataPipeline::new(
            state.clone(),
            args.units.unwrap_or_default(),
            args.indices,
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
        }
    }
}

impl Drop for Dash {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

fn generate_time_markers(window_size: u16, state_len: usize) -> Vec<Span<'static>> {
    let time_labels = (1..)
        .map(|i| i * 30)
        .take_while(|&t| t <= window_size - 5)
        .collect::<Vec<_>>();
    time_labels
        .iter()
        .scan(0, |last_label_len, &time| {
            let pos = window_size - time - 1;
            if pos < window_size {
                let time_marker = format!("{time}s");
                let time_marker_len = time_marker.len() + 1;
                let spacing = "─".repeat(30 * state_len - *last_label_len);
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
        let window_size = (area.width - 1) / state.len() as u16;

        let span_vec = generate_time_markers(window_size, state.len());

        let mut chart = BarChart::default()
            .bar_set(self.bar_set.clone())
            .bar_gap(0)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .title(Line::from(if self.is_paused.load(Ordering::Relaxed) {
                        "Group Chart ⏸"
                    } else {
                        "Group Chart"
                    }).right_aligned()) // Add chart title
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

        let _bars = &(0..window_size)
            .map(|i| {
                BarGroup::default().bars(
                    &(0..state.len())
                        .map(|n| {
                            let state_n = &state[n];
                            let value =
                                state_n.data[state_n.data.len().saturating_sub((i + 1).into())];
                            Bar::default()
                                .value(value as u64)
                                .text_value("".to_owned())
                                .style(Style::default().fg(color_map[n % color_map.len()]))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .rev()
            .for_each(|bar_group| {
                chart = chart.clone().data(bar_group.clone());
            });

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
        let chart_state = &state.data;
        let width = area.width - 1;
        let start = chart_state.len().saturating_sub(width as usize);
        let bars = chart_state[start..]
            .iter()
            .map(|&value| Bar::default().value(value as u64).text_value("".to_owned()))
            .collect::<Vec<_>>();

        let span_vec = generate_time_markers(width, 1);
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
        if n % i == 0 {
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
                                    .find(|&i| num_chart_states % i == 0)
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
                                    .find(|&i| num_chart_states % i == 0)
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
