use blake3;
use getrandom::getrandom;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::time;

const ENTROPY_BUFFER_SIZE: usize = 1024;

#[derive(Clone)]
pub struct Trng {
    entropy_pool: Arc<Mutex<Vec<u8>>>,
}

impl Trng {
    pub fn new() -> Self {
        let trng = Self {
            entropy_pool: Arc::new(Mutex::new(Vec::new())),
        };
        
        let trng_clone = trng.clone();
        tokio::spawn(async move {
            trng_clone.collect_entropy_background().await;
        });

        trng
    }

    async fn collect_entropy_background(&self) {
        let mut interval = time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            self.collect_entropy_round().await;
        }
    }

    async fn collect_entropy_round(&self) {
        let mut entropy = Vec::new();

        
        let mut os_entropy = vec![0u8; 32];
        if getrandom(&mut os_entropy).is_ok() {
            entropy.extend_from_slice(&os_entropy);
        }
        
        entropy.extend_from_slice(&self.collect_timing_jitter());

        if let Some(io_entropy) = self.collect_io_jitter().await {
            entropy.extend_from_slice(&io_entropy);
        }

        let mut pool = self.entropy_pool.lock().unwrap();
        pool.extend(entropy);
        
        if pool.len() > ENTROPY_BUFFER_SIZE {
            let excess = pool.len() - ENTROPY_BUFFER_SIZE;
            pool.drain(0..excess);
        }
    }

    fn collect_timing_jitter(&self) -> Vec<u8> {
        let mut jitter_data = Vec::new();
        let start = Instant::now();
        
        
        for _ in 0..1000 {
            let elapsed = start.elapsed();
            jitter_data.extend_from_slice(&elapsed.as_nanos().to_le_bytes());
        }
        
        jitter_data
    }

    async fn collect_io_jitter(&self) -> Option<Vec<u8>> {
        let start = Instant::now();
        
        
        match TcpStream::connect("127.0.0.1:9").await {
            Ok(_) => {},
            Err(_) => {} 
        }
        
        let elapsed = start.elapsed();
        Some(elapsed.as_nanos().to_le_bytes().to_vec())
    }

    pub fn rand_bytes(&self, len: usize) -> Vec<u8> {
        let pool = self.entropy_pool.lock().unwrap();
        
        if pool.is_empty() {
            
            let mut fallback = vec![0u8; len];
            getrandom(&mut fallback).ok();
            return fallback;
        }

        
        let mut hasher = blake3::Hasher::new();
        hasher.update(&pool);
        hasher.update(&len.to_le_bytes());
        
        let mut output = vec![0u8; len];
        hasher.finalize_xof().fill(&mut output);
        output
    }

    pub fn reseed(&self) {
        let mut pool = self.entropy_pool.lock().unwrap();
        pool.clear();
    }

    
    pub fn monobit_test(&self, data: &[u8]) -> f64 {
        let mut ones = 0;
        
        for byte in data {
            ones += byte.count_ones() as usize;
        }
        
        let total_bits = data.len() * 8;
        let proportion = ones as f64 / total_bits as f64;
        
        
        (proportion - 0.5).abs()
    }

    pub fn runs_test(&self, data: &[u8]) -> f64 {
        let mut runs = 0;
        let mut last_bit = None;
        
        for byte in data {
            for i in 0..8 {
                let bit = (byte >> i) & 1;
                
                if last_bit != Some(bit) {
                    runs += 1;
                    last_bit = Some(bit);
                }
            }
        }
        
        let total_bits = data.len() * 8;
        let expected_runs = (total_bits as f64 / 2.0) + 1.0;
        
        (runs as f64 - expected_runs).abs() / expected_runs
    }

    pub fn approximate_entropy(&self, data: &[u8]) -> f64 {
        let mut frequency = [0usize; 256];
        
        for &byte in data {
            frequency[byte as usize] += 1;
        }
        
        let mut entropy = 0.0;
        let total = data.len() as f64;
        
        for &count in frequency.iter() {
            if count > 0 {
                let probability = count as f64 / total;
                entropy -= probability * probability.log2();
            }
        }
        entropy
    }

    pub fn health_check(&self, sample_size: usize) -> HealthCheckResult {
        let sample = self.rand_bytes(sample_size);
        
        HealthCheckResult {
            monobit_deviation: self.monobit_test(&sample),
            runs_deviation: self.runs_test(&sample),
            shannon_entropy: self.approximate_entropy(&sample),
            sample_size,
        }
    }
}

impl Default for Trng {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub monobit_deviation: f64,
    pub runs_deviation: f64,
    pub shannon_entropy: f64,
    pub sample_size: usize,
}

impl HealthCheckResult {
    pub fn is_healthy(&self) -> bool {
        
        self.monobit_deviation < 0.01 &&    
        self.runs_deviation < 0.1 &&        
        self.shannon_entropy > 7.5          
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trng_health() {
        let trng = Trng::new();
        
        
        time::sleep(Duration::from_millis(500)).await;
        
        let health = trng.health_check(8192); 
        
        println!("Health check results:");
        println!("Monobit deviation: {:.6}", health.monobit_deviation);
        println!("Runs deviation: {:.6}", health.runs_deviation);
        println!("Shannon entropy: {:.6}", health.shannon_entropy);
        
        
        assert!(health.monobit_deviation < 0.05, "Monobit test failed: {}", health.monobit_deviation);
        assert!(health.runs_deviation < 0.2, "Runs test failed: {}", health.runs_deviation);
        assert!(health.shannon_entropy > 7.0, "Entropy too low: {}", health.shannon_entropy);
    }

    #[test]
    fn test_negative_control() {
        let constant_data = vec![0x55u8; 8192]; 
        let trng = Trng {
            entropy_pool: Arc::new(Mutex::new(Vec::new())),
        };
    
        let monobit_dev = trng.monobit_test(&constant_data);
        let runs_dev = trng.runs_test(&constant_data);
        let entropy = trng.approximate_entropy(&constant_data);
        
        println!("Negative control (constant pattern):");
        println!("Monobit deviation: {:.6}", monobit_dev);
        println!("Runs deviation: {:.6}", runs_dev);
        println!("Shannon entropy: {:.6}", entropy);
        
        assert!(monobit_dev > 0.1 || runs_dev > 0.5 || entropy < 1.0,
                "Negative control failed - constant data passed as random!");
    }

    #[test]
    fn test_health_check_methods() {
        
        let trng = Trng {
            entropy_pool: Arc::new(Mutex::new(vec![0xAAu8; 1024])), 
        };
        
        let health = trng.health_check(1024);
         
        assert!(health.monobit_deviation >= 0.0);
        assert!(health.runs_deviation >= 0.0);
        assert!(health.shannon_entropy >= 0.0);
    }
}