use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub step: u64,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub epoch: Option<usize>,
}

const MAX_IN_MEMORY_POINTS: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    pub name: String,
    pub values: Vec<MetricPoint>,
}

impl MetricSeries {
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: Vec::new(),
        }
    }

    pub fn record(&mut self, value: f64, step: u64) {
        self.values.push(MetricPoint {
            step,
            value,
            timestamp: Utc::now(),
            epoch: None,
        });
        self.trim_if_needed();
    }

    pub fn record_with_epoch(&mut self, value: f64, step: u64, epoch: usize) {
        self.values.push(MetricPoint {
            step,
            value,
            timestamp: Utc::now(),
            epoch: Some(epoch),
        });
        self.trim_if_needed();
    }

    fn trim_if_needed(&mut self) {
        if self.values.len() > MAX_IN_MEMORY_POINTS {
            let keep = MAX_IN_MEMORY_POINTS / 2;
            self.values = self.values.split_off(self.values.len() - keep);
        }
    }

    pub fn latest(&self) -> Option<&MetricPoint> {
        self.values.last()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn min(&self) -> Option<f64> {
        self.values.iter().map(|p| p.value).reduce(f64::min)
    }

    pub fn max(&self) -> Option<f64> {
        self.values.iter().map(|p| p.value).reduce(f64::max)
    }

    pub fn last_n(&self, n: usize) -> &[MetricPoint] {
        let start = self.values.len().saturating_sub(n);
        &self.values[start..]
    }

    pub fn downsample_lttb(&self, threshold: usize) -> Vec<MetricPoint> {
        if self.values.len() <= threshold || threshold < 3 {
            return self.values.clone();
        }

        let data = &self.values;
        let mut sampled = Vec::with_capacity(threshold);

        sampled.push(data[0].clone());

        let bucket_size = (data.len() - 2) as f64 / (threshold - 2) as f64;

        let mut a = 0usize;
        for i in 0..(threshold - 2) {
            let avg_start = ((i as f64) * bucket_size).floor() as usize + 1;
            let avg_end = (((i + 1) as f64) * bucket_size).floor() as usize + 1;
            let avg_end = avg_end.min(data.len() - 1);

            let mut avg_x = 0.0f64;
            let mut avg_y = 0.0f64;
            let mut count = 0usize;
            for j in avg_start..=avg_end {
                avg_x += data[j].step as f64;
                avg_y += data[j].value;
                count += 1;
            }
            if count > 0 {
                avg_x /= count as f64;
                avg_y /= count as f64;
            }

            let range_start = ((i as f64) * bucket_size).floor() as usize + 1;
            let range_end = (((i + 1) as f64) * bucket_size).floor() as usize + 1;
            let range_end = range_end.min(data.len() - 1);

            let ax = data[a].step as f64;
            let ay = data[a].value;

            let mut max_area = f64::NEG_INFINITY;
            let mut max_idx = range_start;

            for j in range_start..=range_end {
                let area = ((ax - avg_x) * (data[j].value - ay)
                    - (ax - data[j].step as f64) * (avg_y - ay))
                    .abs();
                if area > max_area {
                    max_area = area;
                    max_idx = j;
                }
            }

            sampled.push(data[max_idx].clone());
            a = max_idx;
        }

        sampled.push(data[data.len() - 1].clone());
        sampled
    }

    pub fn smooth_ema(&self, alpha: f64) -> Vec<MetricPoint> {
        if self.values.is_empty() || alpha <= 0.0 || alpha >= 1.0 {
            return self.values.clone();
        }

        let mut result = Vec::with_capacity(self.values.len());
        let mut smoothed = self.values[0].value;

        for point in &self.values {
            smoothed = alpha * point.value + (1.0 - alpha) * smoothed;
            result.push(MetricPoint {
                step: point.step,
                value: smoothed,
                timestamp: point.timestamp,
                epoch: point.epoch,
            });
        }

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsTimeline {
    series: HashMap<String, MetricSeries>,
}

impl MetricsTimeline {
    pub fn new() -> Self {
        Self {
            series: HashMap::new(),
        }
    }

    pub fn record(&mut self, name: &str, value: f64, step: u64) {
        self.series
            .entry(name.to_string())
            .or_insert_with(|| MetricSeries::new(name.to_string()))
            .record(value, step);
    }

    pub fn record_with_epoch(&mut self, name: &str, value: f64, step: u64, epoch: usize) {
        self.series
            .entry(name.to_string())
            .or_insert_with(|| MetricSeries::new(name.to_string()))
            .record_with_epoch(value, step, epoch);
    }

    pub fn get_series(&self, name: &str) -> Option<&MetricSeries> {
        self.series.get(name)
    }

    pub fn get_series_mut(&mut self, name: &str) -> Option<&mut MetricSeries> {
        self.series.get_mut(name)
    }

    pub fn series_names(&self) -> Vec<String> {
        self.series.keys().cloned().collect()
    }

    pub fn all_series(&self) -> &HashMap<String, MetricSeries> {
        &self.series
    }

    pub fn insert_series(&mut self, name: String, series: MetricSeries) {
        self.series.insert(name, series);
    }

    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    pub fn total_points(&self) -> usize {
        self.series.values().map(|s| s.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_series_record() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(0.5, 1);
        series.record(0.3, 2);
        series.record(0.1, 3);

        assert_eq!(series.len(), 3);
        assert_eq!(series.latest().unwrap().value, 0.1);
        assert_eq!(series.latest().unwrap().step, 3);
    }

    #[test]
    fn test_metric_series_record_with_epoch() {
        let mut series = MetricSeries::new("accuracy".to_string());
        series.record_with_epoch(0.8, 1, 1);
        series.record_with_epoch(0.9, 2, 2);

        assert_eq!(series.len(), 2);
        assert_eq!(series.values[0].epoch, Some(1));
        assert_eq!(series.values[1].epoch, Some(2));
    }

    #[test]
    fn test_metric_series_min_max() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(0.5, 1);
        series.record(0.1, 2);
        series.record(0.3, 3);

        assert_eq!(series.min(), Some(0.1));
        assert_eq!(series.max(), Some(0.5));
    }

    #[test]
    fn test_metric_series_last_n() {
        let mut series = MetricSeries::new("loss".to_string());
        for i in 0..10 {
            series.record(i as f64, i);
        }

        let last = series.last_n(3);
        assert_eq!(last.len(), 3);
        assert_eq!(last[0].value, 7.0);
        assert_eq!(last[2].value, 9.0);
    }

    #[test]
    fn test_metric_series_trim() {
        let mut series = MetricSeries::new("loss".to_string());
        for i in 0..2500 {
            series.record(i as f64, i);
        }

        assert!(series.len() <= 2000);
        assert!(series.values[0].step > 0);
        assert!(series.latest().unwrap().step == 2499);
    }

    #[test]
    fn test_metrics_timeline_record() {
        let mut timeline = MetricsTimeline::new();
        timeline.record("loss", 0.5, 1);
        timeline.record("loss", 0.3, 2);
        timeline.record("accuracy", 0.8, 1);

        assert_eq!(timeline.series_names().len(), 2);
        assert_eq!(timeline.get_series("loss").unwrap().len(), 2);
        assert_eq!(timeline.get_series("accuracy").unwrap().len(), 1);
    }

    #[test]
    fn test_metrics_timeline_record_with_epoch() {
        let mut timeline = MetricsTimeline::new();
        timeline.record_with_epoch("loss", 0.5, 1, 1);
        timeline.record_with_epoch("loss", 0.3, 2, 2);

        let series = timeline.get_series("loss").unwrap();
        assert_eq!(series.values[0].epoch, Some(1));
    }

    #[test]
    fn test_metrics_timeline_total_points() {
        let mut timeline = MetricsTimeline::new();
        timeline.record("loss", 0.5, 1);
        timeline.record("loss", 0.3, 2);
        timeline.record("accuracy", 0.8, 1);

        assert_eq!(timeline.total_points(), 3);
    }

    #[test]
    fn test_metrics_timeline_empty() {
        let timeline = MetricsTimeline::new();
        assert!(timeline.is_empty());
        assert_eq!(timeline.total_points(), 0);
    }

    #[test]
    fn test_metric_point_timestamps_unique() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(0.5, 1);
        std::thread::sleep(std::time::Duration::from_millis(10));
        series.record(0.3, 2);

        assert!(series.values[1].timestamp > series.values[0].timestamp);
    }

    #[test]
    fn test_lttb_downsample_small_data() {
        let mut series = MetricSeries::new("loss".to_string());
        for i in 0..5 {
            series.record(i as f64, i);
        }

        let result = series.downsample_lttb(10);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_lttb_downsample_preserves_endpoints() {
        let mut series = MetricSeries::new("loss".to_string());
        for i in 0..100 {
            series.record(i as f64, i);
        }

        let result = series.downsample_lttb(10);
        assert_eq!(result.first().unwrap().step, 0);
        assert_eq!(result.last().unwrap().step, 99);
        assert!(result.len() <= 10);
    }

    #[test]
    fn test_lttb_downsample_threshold_too_small() {
        let mut series = MetricSeries::new("loss".to_string());
        for i in 0..100 {
            series.record(i as f64, i);
        }

        let result = series.downsample_lttb(2);
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ema_smooth() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(1.0, 0);
        series.record(2.0, 1);
        series.record(3.0, 2);

        let result = series.smooth_ema(0.5);
        assert_eq!(result.len(), 3);
        assert!((result[0].value - 1.0).abs() < 1e-6);
        assert!((result[1].value - 1.5).abs() < 1e-6);
        assert!((result[2].value - 2.25).abs() < 1e-6);
    }

    #[test]
    fn test_ema_smooth_invalid_alpha() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(1.0, 0);
        series.record(2.0, 1);

        let result = series.smooth_ema(0.0);
        assert_eq!(result.len(), 2);
        assert!((result[0].value - 1.0).abs() < 1e-6);

        let result2 = series.smooth_ema(1.0);
        assert_eq!(result2.len(), 2);
        assert!((result2[0].value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ema_smooth_preserves_step() {
        let mut series = MetricSeries::new("loss".to_string());
        series.record(1.0, 10);
        series.record(2.0, 20);

        let result = series.smooth_ema(0.3);
        assert_eq!(result[0].step, 10);
        assert_eq!(result[1].step, 20);
    }
}
