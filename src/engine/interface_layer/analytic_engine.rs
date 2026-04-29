// this file is dedicated to aggregating and making more useful analytics collected from pipeline nodes. Rolling averages, time calculations, etc

use std::collections::VecDeque;

pub struct RollingAverage {
    window: VecDeque<f64>,
    sum: f64
}
impl RollingAverage {
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::from(vec![0.0; window_size]),
            sum: 0.0
        }
    }
    
    pub fn push(&mut self, value: f64) -> f64 {
        let subtractor = self.window.pop_front();
        self.sum += value - subtractor.unwrap_or(0.0);
        self.window.push_back(value);
        
        let avg = self.sum / self.window.len() as f64;
        avg
    }
}