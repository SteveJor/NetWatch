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

        loop {
            // Verifier si on doit s arreter
            if signal_arret.try_recv().is_ok() {
                println!("[collecteur] Arret propre.");
                break;
            }

            // Rafraichir les donnees systeme
            sys.refresh_all();
            sys.refresh_networks();

            // --- Collecter les donnees reseau ---
            let mut total_rx: u64 = 0;
            let mut total_tx: u64 = 0;
            let mut interfaces = Vec::new();

            for (nom, donnees) in sys.networks() {
                // Ignorer l interface de loopback (lo)
                if nom.starts_with("lo") || nom.to_lowercase().contains("loopback") {
                    continue;
                }
                let rx = donnees.received();
                let tx = donnees.transmitted();
                total_rx += rx;
                total_tx += tx;
                interfaces.push(StatInterface {
                    nom: nom.clone(),
                    rx_octets: rx,
                    tx_octets: tx,
                });
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
