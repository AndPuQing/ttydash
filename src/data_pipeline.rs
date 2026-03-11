use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::{interval, Duration, MissedTickBehavior},
};

#[derive(Debug, Clone)]
pub struct DashState {
    data: Vec<f64>,
    pub unit: String,
    pub length: usize,
    next_index: usize,
    running_sum: f64,
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
            next_index: 0,
            running_sum: 0.0,
            min_value: f64::INFINITY,
            max_value: f64::NEG_INFINITY,
            average: 0.0,
        }
    }

    fn oldest_index(&self) -> usize {
        if self.length == self.data.len() {
            self.next_index
        } else {
            0
        }
    }

    fn recalculate_min_max(&mut self) {
        if self.length == 0 {
            self.min_value = f64::INFINITY;
            self.max_value = f64::NEG_INFINITY;
            return;
        }

        let oldest_index = self.oldest_index();
        let first = self.data[oldest_index];
        self.min_value = first;
        self.max_value = first;
        for offset in 1..self.length {
            let value = self.data[(oldest_index + offset) % self.data.len()];
            self.min_value = self.min_value.min(value);
            self.max_value = self.max_value.max(value);
        }
    }

    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        let oldest_index = self.oldest_index();
        (0..self.length).map(move |offset| self.data[(oldest_index + offset) % self.data.len()])
    }

    pub fn recent_values(&self, limit: usize) -> impl Iterator<Item = f64> + '_ {
        let skip = self.length.saturating_sub(limit);
        self.values().skip(skip)
    }

    pub fn value_from_end(&self, offset: usize) -> Option<f64> {
        if offset == 0 || offset > self.length {
            return None;
        }

        let oldest_index = self.oldest_index();
        let index = (oldest_index + self.length - offset) % self.data.len();
        Some(self.data[index])
    }

    pub fn update(&mut self, value: f64) {
        if self.data.is_empty() {
            return;
        }

        let replaced = if self.length == self.data.len() {
            Some(self.data[self.next_index])
        } else {
            None
        };

        self.data[self.next_index] = value;
        self.next_index = (self.next_index + 1) % self.data.len();
        if self.length < self.data.len() {
            self.length += 1;
        }

        self.running_sum += value - replaced.unwrap_or(0.0);
        self.average = self.running_sum / self.length as f64;

        if self.length == 1 {
            self.min_value = value;
            self.max_value = value;
            return;
        }

        if matches!(replaced, Some(old) if old == self.min_value || old == self.max_value) {
            self.recalculate_min_max();
        } else {
            self.min_value = self.min_value.min(value);
            self.max_value = self.max_value.max(value);
        }
    }
}

impl Default for DashState {
    fn default() -> Self {
        Self::new(200)
    }
}

pub enum Extractor {
    Regex(regex::Regex),
    Unit { unit: String, regex: regex::Regex },
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
        let mut update_interval = interval(Duration::from_millis(self.update_frequency.max(1)));
        update_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut latest_line = None;

        let stop_signal = wait_for_stop(self.stop_signal.clone());
        tokio::pin!(stop_signal);

        loop {
            tokio::select! {
                _ = &mut stop_signal => break,
                line = lines.next_line() => match line {
                    Ok(Some(line)) => latest_line = Some(line),
                    Ok(None) => {
                        self.flush_latest_line(&mut latest_line);
                        break;
                    }
                    Err(_) => break,
                },
                _ = update_interval.tick() => {
                    if self.is_paused.load(Ordering::Relaxed) {
                        continue;
                    }
                    self.flush_latest_line(&mut latest_line);
                }
            }
        }
    }

    fn flush_latest_line(&self, latest_line: &mut Option<String>) {
        let Some(line) = latest_line.take() else {
            return;
        };

        let mut state = self.state.write().unwrap();
        if self.extractors.is_empty() {
            let values: Vec<f64> = line
                .split_whitespace()
                .filter_map(|value_str| value_str.parse::<f64>().ok())
                .collect();
            if state.len() < values.len() {
                state.resize(values.len(), DashState::default());
            }
            for (state_item, value) in state.iter_mut().zip(values) {
                state_item.update(value);
            }
            return;
        }

        let fields = self
            .extractors
            .iter()
            .any(|extractor| matches!(extractor, Extractor::Index(_)))
            .then(|| line.split_whitespace().collect::<Vec<_>>());

        for (i, extractor) in self.extractors.iter().enumerate() {
            let value = match extractor {
                Extractor::Regex(re) => re
                    .captures(&line)
                    .and_then(|caps| caps.name("value"))
                    .and_then(|v| v.as_str().parse::<f64>().ok()),
                Extractor::Unit { regex, .. } => regex
                    .captures(&line)
                    .and_then(|caps| caps.get(1))
                    .and_then(|v| v.as_str().parse::<f64>().ok()),
                Extractor::Index(index) => fields
                    .as_ref()
                    .and_then(|parts| parts.get(*index))
                    .and_then(|value| value.parse::<f64>().ok()),
            };

            if let Some(value) = value {
                if i >= state.len() {
                    state.resize(i + 1, DashState::default());
                }
                state[i].update(value);
                if let Extractor::Unit { unit, .. } = extractor {
                    state[i].unit.clone_from(unit);
                }
            }
        }
    }
}

async fn wait_for_stop(stop_signal: Arc<AtomicBool>) {
    while !stop_signal.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::DashState;

    #[test]
    fn dash_state_tracks_stats_without_nan_on_first_value() {
        let mut state = DashState::new(3);

        state.update(10.0);

        assert_eq!(state.length, 1);
        assert_eq!(state.average, 10.0);
        assert_eq!(state.min_value, 10.0);
        assert_eq!(state.max_value, 10.0);
        assert_eq!(state.values().collect::<Vec<_>>(), vec![10.0]);
    }

    #[test]
    fn dash_state_keeps_recent_values_in_order() {
        let mut state = DashState::new(3);

        state.update(1.0);
        state.update(2.0);
        state.update(3.0);
        state.update(4.0);

        assert_eq!(state.length, 3);
        assert_eq!(state.values().collect::<Vec<_>>(), vec![2.0, 3.0, 4.0]);
        assert_eq!(state.recent_values(2).collect::<Vec<_>>(), vec![3.0, 4.0]);
        assert_eq!(state.value_from_end(1), Some(4.0));
        assert_eq!(state.value_from_end(3), Some(2.0));
        assert_eq!(state.average, 3.0);
        assert_eq!(state.min_value, 2.0);
        assert_eq!(state.max_value, 4.0);
    }
}
