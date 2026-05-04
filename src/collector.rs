use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{NetworkExt, ProcessExt, PidExt, System, SystemExt};

// ============================================================
//  Type partage entre les threads
// ============================================================
pub type SharedStore = Arc<RwLock<VecDeque<Snapshot>>>;

/// 60 secondes d historique maximum
pub const TAILLE_HISTORIQUE: usize = 60;

// ============================================================
//  Structures de donnees
// ============================================================

/// Statistiques d un processus a un instant T
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatProcessus {
    pub pid:     u32,
    pub nom:     String,
    pub cpu_pct: f32,
    pub ram_mb:  f64,
    pub rx_bps:  f64,  // bytes par seconde (reel)
    pub tx_bps:  f64,  // bytes par seconde (reel)
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

/// Création d'un nouveau store vide
pub fn nouveau_store() -> SharedStore {
    Arc::new(RwLock::new(VecDeque::with_capacity(TAILLE_HISTORIQUE)))
}

pub fn demarrer_collecte(
    store: SharedStore,
    signal_arret: mpsc::Receiver<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // Initialiser sysinfo
        let mut sys = System::new_all();
        sys.refresh_all();
        sys.refresh_networks_list();
        sys.refresh_networks();

        // Lire le nom de la machine
        let nom_machine = sys.host_name()
            .unwrap_or_else(|| "inconnu".to_string());

        println!("[collecteur] Demarre sur la machine : {}", nom_machine);

        // On garde en memoire le compteur total de chaque interface
        // pour calculer la DIFFERENCE a chaque cycle.
        let mut compteurs_precedents: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();

        // Heure du cycle precedent pour calculer le temps reel ecoule
        let mut instant_precedent = Instant::now();

        // Premiere lecture des compteurs (valeurs de reference)
        for (nom, donnees) in sys.networks() {
            if nom.starts_with("lo") || nom.to_lowercase().contains("loopback") {
                continue;
            }
            compteurs_precedents.insert(
                nom.clone(),
                (donnees.total_received(), donnees.total_transmitted()),
            );
        }

        // Attendre 1 seconde avant la premiere vraie mesure
        thread::sleep(Duration::from_secs(1));

        loop {
            // Verifier si on doit s arreter
            if signal_arret.try_recv().is_ok() {
                println!("[collecteur] Arret propre.");
                break;
            }

            // Rafraichir les donnees systeme
            sys.refresh_all();
            sys.refresh_networks();

            // Calculer le temps reel ecoule
            let maintenant = Instant::now();
            let secondes_ecoulees = maintenant
                .duration_since(instant_precedent)
                .as_secs_f64()
                .max(0.001);
            instant_precedent = maintenant;

            // --- Collecter les donnees reseau ---
            let mut total_rx: f64 = 0.0;
            let mut total_tx: f64 = 0.0;
            let mut interfaces = Vec::new();

            for (nom, donnees) in sys.networks() {
                // Ignorer l interface de loopback (lo)
                if nom.starts_with("lo") || nom.to_lowercase().contains("loopback") {
                    continue;
                }

                // Compteurs cumulatifs actuels
                let rx_cumul_actuel = donnees.total_received();
                let tx_cumul_actuel = donnees.total_transmitted();

                // Recuperer les compteurs du cycle precedent
                let (rx_cumul_avant, tx_cumul_avant) =
                    compteurs_precedents.get(nom).copied().unwrap_or((0, 0));

                // Delta = difference entre maintenant et avant
                let rx_delta = rx_cumul_actuel.saturating_sub(rx_cumul_avant);
                let tx_delta = tx_cumul_actuel.saturating_sub(tx_cumul_avant);

                // Debit en bytes/seconde
                let rx_bps = rx_delta as f64 / secondes_ecoulees;
                let tx_bps = tx_delta as f64 / secondes_ecoulees;

                total_rx += rx_bps;
                total_tx += tx_bps;

                interfaces.push(StatInterface {
                    nom: nom.clone(),
                    rx_octets: rx_bps as u64,
                    tx_octets: tx_bps as u64,
                });

                // Sauvegarder les compteurs pour le prochain cycle
                compteurs_precedents.insert(
                    nom.clone(),
                    (rx_cumul_actuel, tx_cumul_actuel),
                );
            }

            // --- Collecter les processus ---
            let mut processus: Vec<StatProcessus> = sys.processes()
                .iter()
                .map(|(pid, proc_info)| StatProcessus {
                    pid:    pid.as_u32(),
                    nom:    proc_info.name().to_string(),
                    cpu_pct: proc_info.cpu_usage(),
                    ram_mb: proc_info.memory() as f64 / 1_048_576.0,
                    rx_bps: 0.0,
                    tx_bps: 0.0,
                })
                // Ne garder que les processus qui consomment quelque chose
                .filter(|p| p.cpu_pct > 0.05 || p.ram_mb > 1.0)
                .collect();

            // Trier par CPU (les plus gourmands en premier)
            processus.sort_by(|a, b| {
                b.cpu_pct.partial_cmp(&a.cpu_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Garder seulement les 20 premiers
            processus.truncate(20);

            // Estimer le debit reseau par processus
            // (proportionnel a l usage CPU - approximation)
            let cpu_total: f32 = processus.iter().map(|p| p.cpu_pct).sum::<f32>().max(0.001);
            for proc_info in &mut processus {
                let part = proc_info.cpu_pct as f64 / cpu_total as f64;
                proc_info.rx_bps = total_rx as f64 * part;
                proc_info.tx_bps = total_tx as f64 * part;
            }

            // --- Creer le snapshot ---
            let maintenant = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let snapshot = Snapshot {
                horodatage:   maintenant,
                machine:      nom_machine.clone(),
                processus,
                interfaces,
                total_rx_bps: total_rx as f64,
                total_tx_bps: total_tx as f64,
            };

            // --- Stocker dans le store partage ---
            {
                let mut store = store.write().unwrap();
                // Si l historique est plein, retirer le plus ancien
                if store.len() >= TAILLE_HISTORIQUE {
                    store.pop_front();
                }
                store.push_back(snapshot);
            } // Le verrou est libere ici automatiquement

            // Attendre 1 seconde avant la prochaine collecte
            thread::sleep(Duration::from_secs(1));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nouveau_store_vide() {
        let store = nouveau_store();
        let donnees = store.read().unwrap();
        assert_eq!(donnees.len(), 0, "Un nouveau store doit etre vide");
    }

    #[test]
    fn test_stat_processus_defaut() {
        let stat = StatProcessus::default();
        assert_eq!(stat.pid, 0);
        assert_eq!(stat.nom, "");
        assert_eq!(stat.cpu_pct, 0.0);
    }

    #[test]
    fn test_taille_historique() {
        assert_eq!(TAILLE_HISTORIQUE, 60,
            "L historique doit contenir 60 secondes de donnees");
    }

    #[test]
    fn test_snapshot_serialisation() {
        let snap = Snapshot {
            horodatage:   1000,
            machine:      "PC-Test".to_string(),
            processus:    vec![],
            interfaces:   vec![],
            total_rx_bps: 0.0,
            total_tx_bps: 0.0,
        };
        let json = serde_json::to_string(&snap);
        assert!(json.is_ok(), "Le snapshot doit pouvoir etre converti en JSON");
    }
}