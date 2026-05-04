//! Moteur de règles d'alerte — pattern Trait + Génériques (Module 2 GL4).
//!
//! Chaque règle implémente `Regle` et est évaluée à chaque snapshot.
//! Ajouter une règle = créer une struct + implémenter `Regle`.

use crate::{collector::Snapshot, config::Config};
use serde::Serialize;

// ── Modèle d'alerte ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum AlertKind {
    HighBandwidth,
    QuotaWarning,
    QuotaExceeded,
    SuspiciousIp,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub timestamp:  u64,
    pub machine_id: String,
    pub kind:       AlertKind,
    pub message:    String,
}

// ── Trait Regle ────────────────────────────────────────────────────────────

/// Trait extensible : toute règle analyse un snapshot + le quota consommé
/// et produit un vecteur d'alertes.
pub trait Regle: Send + Sync {
    fn evaluer(&self, snapshot: &Snapshot, quota_used_mb: u64) -> Vec<Alert>;
}

// ── Règle 1 : Débit élevé ─────────────────────────────────────────────────

pub struct RegleDebit { pub max_mbps: f64 }

impl Regle for RegleDebit {
    fn evaluer(&self, snap: &Snapshot, _: u64) -> Vec<Alert> {
        let seuil = self.max_mbps * 1_000_000.0;
        snap.processes.iter()
            .filter(|p| p.rx_rate + p.tx_rate > seuil)
            .map(|p| Alert {
                timestamp:  snap.timestamp,
                machine_id: snap.machine_id.clone(),
                kind:       AlertKind::HighBandwidth,
                message:    format!(
                    "⚡ {} utilise {:.1} MB/s (seuil : {:.1} MB/s)",
                    p.name,
                    (p.rx_rate + p.tx_rate) / 1_000_000.0,
                    self.max_mbps
                ),
            })
            .collect()
    }
}

// ── Règle 2 : Quota ────────────────────────────────────────────────────────

pub struct RegleQuota {
    pub machine:      String,
    pub limit_mb:     u64,
    pub warn_percent: u64,
}

impl Regle for RegleQuota {
    fn evaluer(&self, snap: &Snapshot, quota_used_mb: u64) -> Vec<Alert> {
        if snap.machine_id != self.machine || self.limit_mb == 0 { return vec![]; }

        let pct = (quota_used_mb * 100).saturating_div(self.limit_mb);

        if quota_used_mb >= self.limit_mb {
            vec![Alert {
                timestamp:  snap.timestamp,
                machine_id: snap.machine_id.clone(),
                kind:       AlertKind::QuotaExceeded,
                message:    format!(
                    "🚫 Quota dépassé sur {} : {} MB / {} MB ({}%)",
                    self.machine, quota_used_mb, self.limit_mb, pct
                ),
            }]
        } else if pct >= self.warn_percent {
            vec![Alert {
                timestamp:  snap.timestamp,
                machine_id: snap.machine_id.clone(),
                kind:       AlertKind::QuotaWarning,
                message:    format!(
                    "⚠️  Quota à {}% sur {} : {} MB / {} MB",
                    pct, self.machine, quota_used_mb, self.limit_mb
                ),
            }]
        } else {
            vec![]
        }
    }
}

// ── Règle 3 : IP suspecte ──────────────────────────────────────────────────

pub struct RegleIpSuspecte { pub ips: Vec<String> }

impl Regle for RegleIpSuspecte {
    fn evaluer(&self, _snap: &Snapshot, _: u64) -> Vec<Alert> {
        // Extension : croiser avec /proc/net/tcp sur Linux
        vec![]
    }
}

// ── Fabrique ───────────────────────────────────────────────────────────────

/// Construit la liste des règles actives à partir de la configuration.
pub fn build_rules(cfg: &Config) -> Vec<Box<dyn Regle>> {
    let mut rules: Vec<Box<dyn Regle>> = vec![
        Box::new(RegleDebit { max_mbps: cfg.alert_rules.max_rate_mbps }),
    ];
    for q in &cfg.quotas {
        rules.push(Box::new(RegleQuota {
            machine:      q.machine.clone(),
            limit_mb:     q.limit_mb,
            warn_percent: cfg.alert_rules.quota_warn_percent,
        }));
    }
    if !cfg.alert_rules.suspicious_ips.is_empty() {
        rules.push(Box::new(RegleIpSuspecte {
            ips: cfg.alert_rules.suspicious_ips.clone(),
        }));
    }
    rules
}
