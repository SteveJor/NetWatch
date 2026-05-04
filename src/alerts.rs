use crate::collector::Snapshot;
use crate::config::Config;
use serde::Serialize;

// ============================================================
//  Structures d alerte
// ============================================================

/// Type d alerte - decrit la cause
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TypeAlerte {
    /// Un processus consomme trop de bande passante
    DebitEleve,
    /// Le quota de la machine est presque atteint
    AvertissementQuota,
    /// Le quota de la machine est depasse
    QuotaDepasse,
}

/// Une alerte generee par le moteur de regles
#[derive(Debug, Clone, Serialize)]
pub struct Alerte {
    /// Heure de declenchement (secondes depuis epoch)
    pub horodatage: u64,
    /// Nom de la machine concernee
    pub machine: String,
    /// Type de l alerte
    pub type_alerte: TypeAlerte,
    /// Message lisible par un humain
    pub message: String,
}


/// Interface commune pour toutes les regles d alerte
/// Send + Sync = peut etre utilise en securite entre threads
pub trait RegleAlerte: Send + Sync {
    /// Analyse un snapshot et retourne les alertes generees
    /// quota_utilise_mb = megaoctets consommes depuis le dernier reset
    fn evaluer(&self, snapshot: &Snapshot, quota_utilise_mb: u64) -> Vec<Alerte>;
}

// ============================================================
//  Regle 1 : Debit eleve par processus
// ============================================================

/// Alerte si un processus depasse le seuil de debit configure
pub struct RegleDebit {
    /// Seuil en MB/s au-dela duquel l alerte est declenchee
    pub seuil_mbps: f64,
}

impl RegleAlerte for RegleDebit {
    fn evaluer(&self, snapshot: &Snapshot, _quota: u64) -> Vec<Alerte> {
        let seuil_bps = self.seuil_mbps * 1_000_000.0;
        let mut alertes = Vec::new();

        for proc_info in &snapshot.processus {
            let debit_total = proc_info.rx_bps + proc_info.tx_bps;
            if debit_total > seuil_bps {
                alertes.push(Alerte {
                    horodatage:  snapshot.horodatage,
                    machine:     snapshot.machine.clone(),
                    type_alerte: TypeAlerte::DebitEleve,
                    message: format!(
                        "⚡ {} utilise {:.1} MB/s (seuil : {:.1} MB/s)",
                        proc_info.nom,
                        debit_total / 1_000_000.0,
                        self.seuil_mbps
                    ),
                });
            }
        }
        alertes
    }
}

// ============================================================
//  Regle 2 : Quota de donnees
// ============================================================

/// Alerte quand le quota d une machine est presque atteint ou depasse
pub struct RegleQuota {
    /// Nom de la machine concernee
    pub machine: String,
    /// Limite en megaoctets
    pub limite_mb: u64,
    /// Pourcentage de debit pour l avertissement (ex: 80)
    pub seuil_pct: u64,
}

impl RegleAlerte for RegleQuota {
    fn evaluer(&self, snapshot: &Snapshot, quota_utilise_mb: u64) -> Vec<Alerte> {
        // Cette regle ne s applique qu a la machine configuree
        if snapshot.machine != self.machine || self.limite_mb == 0 {
            return Vec::new();
        }

        // Calculer le pourcentage utilise
        let pct = (quota_utilise_mb * 100) / self.limite_mb;

        if quota_utilise_mb >= self.limite_mb {
            // Quota depasse
            vec![Alerte {
                horodatage:  snapshot.horodatage,
                machine:     snapshot.machine.clone(),
                type_alerte: TypeAlerte::QuotaDepasse,
                message: format!(
                    "🚫 Quota depasse sur {} : {} MB / {} MB ({}%)",
                    self.machine, quota_utilise_mb, self.limite_mb, pct
                ),
            }]
        } else if pct >= self.seuil_pct {
            // Avertissement quota bientot atteint
            vec![Alerte {
                horodatage:  snapshot.horodatage,
                machine:     snapshot.machine.clone(),
                type_alerte: TypeAlerte::AvertissementQuota,
                message: format!(
                    "⚠️ Quota a {}% sur {} : {} MB / {} MB",
                    pct, self.machine, quota_utilise_mb, self.limite_mb
                ),
            }]
        } else {
            Vec::new()
        }
    }
}

// ============================================================
//  Fabrique de regles
// ============================================================

/// Cree la liste des regles actives depuis la configuration
pub fn creer_regles(config: &Config) -> Vec<Box<dyn RegleAlerte>> {
    let mut regles: Vec<Box<dyn RegleAlerte>> = Vec::new();

    // Regle de debit (toujours active)
    regles.push(Box::new(RegleDebit {
        seuil_mbps: config.alertes.debit_max_mbps,
    }));

    // Regles de quota (une par machine configuree)
    for quota in &config.quotas {
        regles.push(Box::new(RegleQuota {
            machine:    quota.machine.clone(),
            limite_mb:  quota.limite_mb,
            seuil_pct:  config.alertes.avertissement_quota_pct,
        }));
    }

    regles
}
