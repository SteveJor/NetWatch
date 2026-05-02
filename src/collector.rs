use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;
use std::time::Duration;
use sysinfo::{NetworkExt, ProcessExt, PidExt, System, SystemExt};


pub type SharedStore = Arc<RwLock<VecDeque<Snapshot>>>;

pub const TAILLE_HISTORIQUE: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatProcessus {
    pub pid: u32,
    pub nom: String,
    pub cpu_pct: f32,
    pub ram_mb: f64,
    pub rx_bps: f64,
    pub tx_bps: f64,
}

/// Statistiques d'une interface reseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatInterface {
    pub nom: String,
    pub rx_octets: u64,
    pub tx_octets: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub horodatage: u64,
    pub machine: String,
    pub processus: Vec<StatProcessus>,
    pub interfaces: Vec<StatInterface>,
    pub total_rx_bps: f64,
    pub total_tx_bps: f64,
}

