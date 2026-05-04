use crate::alerts::{self, Alerte};
use crate::collector::{self, SharedStore, Snapshot};
use crate::config::Config;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

// ============================================================
//  Etat global partage entre toutes les taches async
// ============================================================

/// Toutes les donnees affichees sur le dashboard
#[derive(Debug, Clone, Serialize, Default)]
struct EtatDashboard {
    /// Dernier snapshot de la machine locale
    local: Option<Snapshot>,
    /// Etat de chaque agent (nom -> statut)
    agents: HashMap<String, StatutAgent>,
    /// Les 100 dernieres alertes
    alertes: Vec<Alerte>,
    /// Etat des quotas par machine
    quotas: HashMap<String, EtatQuota>,
    /// Historique du debit entrant (60 points)
    historique_rx: Vec<f64>,
    /// Historique du debit sortant (60 points)
    historique_tx: Vec<f64>,
}

/// Statut d un agent distant
#[derive(Debug, Clone, Serialize)]
struct StatutAgent {
    nom:      String,
    ip:       String,
    en_ligne: bool,
    snapshot: Option<Snapshot>,
}

/// Etat du quota d une machine
#[derive(Debug, Clone, Serialize)]
pub struct EtatQuota {
    pub machine:    String,
    pub utilise_mb: u64,
    pub limite_mb:  u64,
    pub pct:        u64,
}

/// Contient toutes les structures partagees entre les taches
#[derive(Clone)]
struct EtatApp {
    store:           SharedStore,
    config:          Arc<RwLock<Config>>,
    dashboard:       Arc<RwLock<EtatDashboard>>,
    quota_octets:    Arc<RwLock<HashMap<String, u64>>>,
}

// ============================================================
//  Point d entree du mode maitre
// ============================================================

pub async fn lancer() {
    let config     = Config::charger();
    let port_web   = config.port_web;
    let store      = collector::nouveau_store();
    let config     = Arc::new(RwLock::new(config));
    let dashboard  = Arc::new(RwLock::new(EtatDashboard::default()));
    let quota_octets: Arc<RwLock<HashMap<String, u64>>> = Arc::new(RwLock::new(HashMap::new()));

    // Canal d arret pour le collecteur local
    let (signal_tx, signal_rx) = mpsc::channel::<()>();
    let _collecteur = collector::demarrer_collecte(store.clone(), signal_rx);

    let etat = EtatApp { store, config, dashboard, quota_octets };

    // Lancer les taches asynchrones en parallele
    tokio::spawn(tache_interroger_agents(etat.clone()));
    tokio::spawn(tache_alertes(etat.clone()));
    tokio::spawn(tache_compteur_quota(etat.clone()));

    // Definir les routes HTTP
    let app = Router::new()
        .route("/",                  get(servir_dashboard))
        .route("/static/styles.css",  get(servir_css))
        .route("/static/charts.js",  get(servir_charts_js))
        .route("/static/settings.js",get(servir_settings_js))
        .route("/static/app.js",     get(servir_app_js))
        .route("/ws",                get(handler_websocket))
        .route("/api/config",        get(api_lire_config).post(api_sauvegarder_config))
        .route("/api/quotas/reset",  post(api_reset_quota))
        .layer(CorsLayer::permissive())
        .with_state(etat);

    let adresse = format!("0.0.0.0:{}", port_web);
    let listener = TcpListener::bind(&adresse).await
        .unwrap_or_else(|_| panic!("Impossible d ecouter sur le port {}", port_web));

    afficher_banniere(port_web);
    ouvrir_navigateur(port_web);

    tokio::select! {
        res = axum::serve(listener, app) => {
            if let Err(e) = res { eprintln!("[maitre] Erreur : {}", e); }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\\n[maitre] Arret...");
            signal_tx.send(()).ok();
        }
    }
}

// ============================================================
//  Routes - Fichiers statiques (HTML, CSS, JS)
// ============================================================

async fn servir_dashboard() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn servir_css() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "text/css")], include_str!("../static/styles.css"))
}

async fn servir_app_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], include_str!("../static/app.js"))
}

async fn servir_charts_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], include_str!("../static/charts.js"))
}

async fn servir_settings_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], include_str!("../static/settings.js"))
}

// ============================================================
//  WebSocket - Envoi des donnees en temps reel
// ============================================================

async fn handler_websocket(ws: WebSocketUpgrade, State(etat): State<EtatApp>) -> Response {
    ws.on_upgrade(|socket| boucle_push(socket, etat))
}

/// Boucle qui envoie l etat du dashboard toutes les secondes
async fn boucle_push(mut socket: WebSocket, etat: EtatApp) {
    loop {
        // Lire l etat courant
        let payload = {
            let dash = etat.dashboard.read().unwrap();
            serde_json::to_string(&*dash).unwrap_or_default()
        };

        // Envoyer au navigateur
        if socket.send(Message::Text(payload)).await.is_err() {
            break; // Client deconnecte
        }

        // Attendre 1 seconde
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ============================================================
//  Taches asynchrones - tournent en permanence en arriere-plan
// ============================================================

/// Interroge tous les agents configured et met a jour le dashboard
async fn tache_interroger_agents(etat: EtatApp) {
    // Client HTTP avec timeout de 2 secondes
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    loop {
        let agents_config = etat.config.read().unwrap().agents.clone();
        let mut statuts = HashMap::new();

        // Interroger chaque agent
        for agent_cfg in &agents_config {
            let url = format!("http://{}:7878/api/snapshot", agent_cfg.ip);
            let snapshot: Option<Snapshot> = client
                .get(&url)
                .send().await.ok()
                .and_then(|r| if r.status().is_success() { Some(r) } else { None })
                .and_then(|r| Some(async move { r.json::<Option<Snapshot>>().await }))
                .map(|f| async move { f.await.ok().flatten() })
                .map(|f| tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(f)
                }))
                .flatten();

            statuts.insert(agent_cfg.nom.clone(), StatutAgent {
                nom:      agent_cfg.nom.clone(),
                ip:       agent_cfg.ip.clone(),
                en_ligne: snapshot.is_some(),
                snapshot,
            });
        }

        // Mettre a jour le dashboard
        {
            let local = etat.store.read().unwrap().back().cloned();
            let (hist_rx, hist_tx) = {
                let store = etat.store.read().unwrap();
                let rx = store.iter().map(|s| s.total_rx_bps).collect();
                let tx = store.iter().map(|s| s.total_tx_bps).collect();
                (rx, tx)
            };

            let mut dash = etat.dashboard.write().unwrap();
            dash.local        = local;
            dash.agents       = statuts;
            dash.historique_rx = hist_rx;
            dash.historique_tx = hist_tx;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Evalue les regles d alerte et emet les notifications
async fn tache_alertes(etat: EtatApp) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let config   = etat.config.read().unwrap().clone();
        let snapshot = etat.store.read().unwrap().back().cloned();

        if let Some(snap) = snapshot {
            let quota_mb = {
                let qo = etat.quota_octets.read().unwrap();
                qo.get(&snap.machine).copied().unwrap_or(0) / 1_000_000
            };

            // Evaluer toutes les regles
            let regles = alerts::creer_regles(&config);
            let nouvelles: Vec<Alerte> = regles.iter()
                .flat_map(|r| r.evaluer(&snap, quota_mb))
                .collect();

            // Envoyer les notifications et stocker les alertes
            if !nouvelles.is_empty() {
                for alerte in &nouvelles {
                    envoyer_notification("NetWatch", &alerte.message);
                }
                let mut dash = etat.dashboard.write().unwrap();
                for alerte in nouvelles {
                    if dash.alertes.len() >= 100 { dash.alertes.remove(0); }
                    dash.alertes.push(alerte);
                }
            }

            // Mettre a jour l etat des quotas
            let qo = etat.quota_octets.read().unwrap();
            let mut quotas = HashMap::new();
            for quota_cfg in &config.quotas {
                let utilise_bytes = qo.get(&quota_cfg.machine).copied().unwrap_or(0);
                let utilise_mb    = utilise_bytes / 1_000_000;
                let pct = if quota_cfg.limite_mb > 0 {
                    (utilise_mb * 100 / quota_cfg.limite_mb).min(100)
                } else { 0 };
                quotas.insert(quota_cfg.machine.clone(), EtatQuota {
                    machine:    quota_cfg.machine.clone(),
                    utilise_mb,
                    limite_mb:  quota_cfg.limite_mb,
                    pct,
                });
            }
            drop(qo);
            etat.dashboard.write().unwrap().quotas = quotas;
        }
    }
}

/// Comptabilise les bytes consommes par machine pour les quotas
async fn tache_compteur_quota(etat: EtatApp) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Some(snap) = etat.store.read().unwrap().back().cloned() {
            let bytes_seconde = snap.total_rx_bps as u64 + snap.total_tx_bps as u64;
            *etat.quota_octets.write().unwrap()
                .entry(snap.machine)
                .or_insert(0) += bytes_seconde;
        }
    }
}

// ============================================================
//  Routes API - Configuration et quotas
// ============================================================

async fn api_lire_config(State(etat): State<EtatApp>) -> Json<Config> {
    Json(etat.config.read().unwrap().clone())
}

async fn api_sauvegarder_config(
    State(etat): State<EtatApp>,
    Json(nouvelle_config): Json<Config>,
) -> Json<serde_json::Value> {
    let ok = nouvelle_config.sauvegarder().is_ok();
    *etat.config.write().unwrap() = nouvelle_config;
    Json(serde_json::json!({
        "ok":      ok,
        "message": if ok { "Configuration sauvegardee" } else { "Erreur de sauvegarde" }
    }))
}

#[derive(Deserialize)]
struct CorpsResetQuota { machine: String }

async fn api_reset_quota(
    State(etat): State<EtatApp>,
    Json(corps): Json<CorpsResetQuota>,
) -> Json<serde_json::Value> {
    etat.quota_octets.write().unwrap().remove(&corps.machine);
    Json(serde_json::json!({ "ok": true }))
}

// ============================================================
//  Utilitaires
// ============================================================

fn ouvrir_navigateur(port: u16) {
    let url = format!("http://localhost:{}", port);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(800));
        #[cfg(target_os = "linux")]
        { std::process::Command::new("xdg-open").arg(&url).spawn().ok(); }
        #[cfg(target_os = "windows")]
        { std::process::Command::new("cmd").args(["/c","start",&url]).spawn().ok(); }
        #[cfg(target_os = "macos")]
        { std::process::Command::new("open").arg(&url).spawn().ok(); }
    });
}

fn envoyer_notification(titre: &str, corps: &str) {
    let corps = corps.to_string();
    let titre = titre.to_string();
    std::thread::spawn(move || {
        notify_rust::Notification::new()
            .summary(&titre)
            .body(&corps)
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show().ok();
    });
}

fn afficher_banniere(port: u16) {
    println!("╔══════════════════════════════════════════╗");
    println!("║       NetWatch — Mode MAITRE             ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Dashboard : http://localhost:{:<5}      ║", port);
    println!("║  Ctrl+C pour arreter                     ║");
    println!("╚══════════════════════════════════════════╝");
}
