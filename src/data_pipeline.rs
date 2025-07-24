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

pub struct DataPipeline {
    state: Arc<RwLock<Vec<DashState>>>,
    units: Vec<String>,
    indices: Option<Vec<usize>>,
    update_frequency: u64,
    stop_signal: Arc<AtomicBool>,
}

impl DataPipeline {
    pub fn new(
        state: Arc<RwLock<Vec<DashState>>>,
        units: Vec<String>,
        indices: Option<Vec<usize>>,
        update_frequency: u64,
        stop_signal: Arc<AtomicBool>,
    ) -> Self {
        Self {
            state,
            units,
            indices,
            update_frequency,
            stop_signal,
        }
    }

    pub async fn run(self) {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        while !self.stop_signal.load(Ordering::Relaxed) {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.update_frequency)).await;
            if let Ok(Some(line)) = lines.next_line().await {
                let mut state = self.state.write().unwrap();
                if !self.units.is_empty() {
                    for (i, unit) in self.units.iter().enumerate() {
                        let unit_str = unit.to_string();
                        let re =
                            regex::Regex::new(&format!(r"(?i)\b(\d+(\.\d+)?)\s*{}\b", unit_str))
                                .unwrap();
                        if let Some(captures) = re.captures(&line) {
                            let value = captures
                                .get(1)
                                .and_then(|v| v.as_str().parse::<f64>().ok())
                                .unwrap_or(0.0);
                            state[i].update(value);
                            state[i].unit = unit_str.to_string();
                        }
                    }
                } else if line.split_whitespace().next().is_some() {
                    let values: Vec<f64> = line
                        .split_whitespace()
                        .filter_map(|value_str| value_str.parse::<f64>().ok())
                        .collect();
                    if let Some(indices) = &self.indices {
                        if state.len() < indices.len() {
                            state.resize(indices.len(), DashState::default());
                        }
                        indices
                            .iter()
                            .filter_map(|&index| values.get(index - 1).copied())
                            .enumerate()
                            .for_each(|(i, value)| state[i].update(value));
                    } else {
                        if state.len() < values.len() {
                            state.resize(values.len(), DashState::default());
                        }
                        state
                            .iter_mut()
                            .zip(values.iter())
                            .for_each(|(state_item, &value)| {
                                state_item.update(value);
                            });
                    }
                }
            }
        }
    }
}
