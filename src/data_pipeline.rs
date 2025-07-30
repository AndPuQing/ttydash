use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone)]
pub struct DashState {
    pub data: Vec<f64>,
    pub unit: String,
    pub length: usize,
    pub min_value: f64,
    pub max_value: f64,
    pub average: f64,
}

impl DashState {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0.0; size],
            unit: String::new(),
            length: 0,
            min_value: f64::INFINITY,
            max_value: f64::NEG_INFINITY,
            average: 0.0,
        }
    }

    fn calculate_stats(&mut self) {
        let data_slice = &self.data[self.data.len() - self.length..];
        let sum: f64 = data_slice.iter().sum();
        let len = data_slice.len() as f64;
        self.average = sum / len;
        self.min_value = data_slice.iter().copied().fold(f64::INFINITY, f64::min);
        self.max_value = data_slice.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    }

    pub fn update(&mut self, value: f64) {
        self.data.rotate_left(1);
        let len = self.data.len();
        self.data[len - 1] = value;
        self.calculate_stats();
        self.length = std::cmp::min(self.length + 1, self.data.len());
    }
}

impl Default for DashState {
    fn default() -> Self {
        Self::new(200)
    }
}

pub enum Extractor {
    Regex(regex::Regex),
    Unit(String),
    Index(usize),
}

pub struct DataPipeline {
    state: Arc<RwLock<Vec<DashState>>>,
    extractors: Vec<Extractor>,
    update_frequency: u64,
    stop_signal: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

impl DataPipeline {
    pub fn new(
        state: Arc<RwLock<Vec<DashState>>>,
        extractors: Vec<Extractor>,
        update_frequency: u64,
        stop_signal: Arc<AtomicBool>,
        is_paused: Arc<AtomicBool>,
    ) -> Self {
        Self {
            state,
            extractors,
            update_frequency,
            stop_signal,
            is_paused,
        }
    }

    pub async fn run(self) {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        while !self.stop_signal.load(Ordering::Relaxed) {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.update_frequency)).await;
            if self.is_paused.load(Ordering::Relaxed) {
                continue;
            }
            if let Ok(Some(line)) = lines.next_line().await {
                let mut state = self.state.write().unwrap();
                if self.extractors.is_empty() {
                    let values: Vec<f64> = line
                        .split_whitespace()
                        .filter_map(|value_str| value_str.parse::<f64>().ok())
                        .collect();
                    if state.len() < values.len() {
                        state.resize(values.len(), DashState::default());
                    }
                    state
                        .iter_mut()
                        .zip(values.iter())
                        .for_each(|(state_item, &value)| {
                            state_item.update(value);
                        });
                } else {
                    for (i, extractor) in self.extractors.iter().enumerate() {
                        let value = match extractor {
                            Extractor::Regex(re) => re
                                .captures(&line)
                                .and_then(|caps| caps.name("value"))
                                .and_then(|v| v.as_str().parse::<f64>().ok()),
                            Extractor::Unit(unit) => {
                                let re = regex::Regex::new(&format!(
                                    r"(?i)\b(\d+(\.\d+)?)\s*{unit}\b"
                                ))
                                .unwrap();
                                re.captures(&line)
                                    .and_then(|caps| caps.get(1))
                                    .and_then(|v| v.as_str().parse::<f64>().ok())
                            }
                            Extractor::Index(index) => line
                                .split_whitespace()
                                .nth(*index)
                                .and_then(|s| s.parse::<f64>().ok()),
                        };
                        if let Some(value) = value {
                            if i >= state.len() {
                                state.resize(i + 1, DashState::default());
                            }
                            state[i].update(value);
                            if let Extractor::Unit(unit) = extractor {
                                state[i].unit = unit.clone();
                            }
                        }
                    }
                }
            }
        }
    }
}
