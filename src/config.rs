//! Lecture et persistance de la configuration `netwatch.toml`.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const CONFIG_PATH: &str = "netwatch.toml";

// ── Structs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_web_port")]
    pub web_port: u16,

    #[serde(default = "default_agent_port")]
    pub agent_port: u16,

    #[serde(default)]
    pub agents: Vec<AgentConfig>,

    #[serde(default)]
    pub quotas: Vec<QuotaConfig>,

    #[serde(default)]
    pub alert_rules: AlertRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub ip:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub machine:  String,
    pub limit_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRules {
    #[serde(default = "default_max_rate")]
    pub max_rate_mbps: f64,

    #[serde(default = "default_quota_warn")]
    pub quota_warn_percent: u64,

    #[serde(default)]
    pub suspicious_ips: Vec<String>,
}

// ── Defaults ───────────────────────────────────────────────────────────────
fn default_web_port()   -> u16  { 3000 }
fn default_agent_port() -> u16  { 7878 }
fn default_max_rate()   -> f64  { 5.0  }
fn default_quota_warn() -> u64  { 80   }

impl Default for AlertRules {
    fn default() -> Self {
        Self {
            max_rate_mbps:      5.0,
            quota_warn_percent: 80,
            suspicious_ips:     vec![],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            web_port:    3000,
            agent_port:  7878,
            agents:      vec![],
            quotas:      vec![],
            alert_rules: AlertRules::default(),
        }
    }
}

// ── Méthodes ───────────────────────────────────────────────────────────────
impl Config {
    /// Charge la configuration depuis `netwatch.toml`, ou retourne les
    /// valeurs par défaut si le fichier est absent / illisible.
    pub fn load() -> Self {
        if Path::new(CONFIG_PATH).exists() {
            let raw = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
            toml::from_str(&raw).unwrap_or_default()
        } else {
            Config::default()
        }
    }

    /// Persiste la configuration courante dans `netwatch.toml`.
    pub fn save(&self) -> std::io::Result<()> {
        let s = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(CONFIG_PATH, s)
    }
}
