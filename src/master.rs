// ============================================================
//  Membre 6 - Serveur Maître
//
//  Ce module orchestre tout le système en mode « maître » :
//  1. Collecte les statistiques locales
//  2. Interroge les agents toutes les secondes
//  3. Analyse les alertes
//  4. Sert le tableau de bord web
//  5. Pousse les données en temps réel via WebSocket
//
//  Rôle architectural :
//  ---------------------
//  Le serveur maître est le point central du système NetWatch.
//  Il agrège les données provenant de la machine locale et de
//  l'ensemble des agents distants configurés, puis les expose
//  via une interface web accessible depuis un navigateur.
//
//  Concurrence :
//  -------------
//  Toutes les tâches de fond (interrogation des agents, alertes,
//  compteur de quota) s'exécutent en parallèle grâce au runtime
//  asynchrone Tokio. L'état partagé est protégé par des verrous
//  en lecture/écriture (RwLock) afin d'éviter les accès concurrents
//  non contrôlés.
//
//  Communication avec le tableau de bord :
//  ----------------------------------------
//  Les données sont poussées vers le navigateur via WebSocket
//  à raison d'une mise à jour par seconde. Aucune action côté
//  client n'est nécessaire pour rafraîchir l'affichage.
// ============================================================

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
//  État global partagé entre toutes les tâches async
// ============================================================

// --- Structure principale du tableau de bord ---
// Toutes les informations affichées sur le tableau de bord sont
// regroupées ici. Cette structure est sérialisable en JSON afin
// d'être envoyée directement au navigateur via WebSocket.
/// Toutes les données affichées sur le dashboard
#[derive(Debug, Clone, Serialize, Default)]
struct EtatDashboard {
    /// Dernier snapshot de la machine locale
    local: Option<Snapshot>,
    /// État de chaque agent (nom -> statut)
    agents: HashMap<String, StatutAgent>,
    /// Les 100 dernières alertes
    alertes: Vec<Alerte>,
    /// État des quotas par machine
    quotas: HashMap<String, EtatQuota>,
    /// Historique du débit entrant (60 points)
    historique_rx: Vec<f64>,
    /// Historique du débit sortant (60 points)
    historique_tx: Vec<f64>,
}

// --- Statut d'un agent distant ---
// Chaque agent interrogé dispose d'un statut indiquant s'il est
// joignable et, le cas échéant, son dernier snapshot réseau.
// Le champ `en_ligne` vaut false dès qu'une requête HTTP échoue
// ou dépasse le délai d'attente configuré (2 secondes).
/// Statut d'un agent distant
#[derive(Debug, Clone, Serialize)]
struct StatutAgent {
    nom:      String,
    ip:       String,
    en_ligne: bool,
    snapshot: Option<Snapshot>,
}

// --- État du quota d'une machine ---
// Utilisé pour suivre la consommation réseau cumulée d'une machine
// par rapport à la limite configurée. Le pourcentage `pct` est
// plafonné à 100 pour éviter les débordements d'affichage.
/// État du quota d'une machine
#[derive(Debug, Clone, Serialize)]
pub struct EtatQuota {
    pub machine:    String,
    pub utilise_mb: u64,
    pub limite_mb:  u64,
    pub pct:        u64,
}

// --- Conteneur d'état de l'application ---
// Cette structure est clonée et transmise à chaque route HTTP et
// à chaque tâche asynchrone. Le clonage est peu coûteux car tous
// les champs sont des Arc (pointeurs à comptage de références).
/// Contient toutes les structures partagées entre les tâches
#[derive(Clone)]
struct EtatApp {
    store:           SharedStore,
    config:          Arc<RwLock<Config>>,
    dashboard:       Arc<RwLock<EtatDashboard>>,
    quota_octets:    Arc<RwLock<HashMap<String, u64>>>,
}

// ============================================================
//  Point d'entrée du mode maître
// ============================================================

// --- Fonction principale du mode maître ---
// Cette fonction asynchrone initialise toutes les ressources
// partagées, démarre les tâches de fond, enregistre les routes
// HTTP et met le serveur web en écoute sur le port configuré.
// Elle se termine proprement à la réception du signal Ctrl+C.
pub async fn lancer() {
    let config     = Config::charger();
    let port_web   = config.port_web;
    let store      = collector::nouveau_store();
    let config     = Arc::new(RwLock::new(config));
    let dashboard  = Arc::new(RwLock::new(EtatDashboard::default()));
    let quota_octets: Arc<RwLock<HashMap<String, u64>>> = Arc::new(RwLock::new(HashMap::new()));

    // Canal d'arrêt pour le collecteur local
    let (signal_tx, signal_rx) = mpsc::channel::<()>();
    let _collecteur = collector::demarrer_collecte(store.clone(), signal_rx);

    let etat = EtatApp { store, config, dashboard, quota_octets };

    // Lancer les tâches asynchrones en parallèle
    tokio::spawn(tache_interroger_agents(etat.clone()));
    tokio::spawn(tache_alertes(etat.clone()));
    tokio::spawn(tache_compteur_quota(etat.clone()));

    // Définir les routes HTTP
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
        .unwrap_or_else(|_| panic!("Impossible d'écouter sur le port {}", port_web));

    afficher_banniere(port_web);
    ouvrir_navigateur(port_web);

    // Attente de la fin du serveur ou du signal d'arrêt Ctrl+C.
    // L'opérateur select! garantit que l'un ou l'autre des deux
    // futurs sera traité dès qu'il sera résolu en premier.
    tokio::select! {
        res = axum::serve(listener, app) => {
            if let Err(e) = res { eprintln!("[maître] Erreur : {}", e); }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n[maître] Arrêt...");
            signal_tx.send(()).ok();
        }
    }
}

// ============================================================
//  Routes - Fichiers statiques (HTML, CSS, JS)
// ============================================================

// --- Service des fichiers statiques ---
// Les fichiers HTML, CSS et JavaScript sont compilés directement
// dans le binaire à l'aide de la macro `include_str!`. Cela
// simplifie le déploiement : aucun répertoire externe n'est requis.
// Chaque route renvoie le type MIME approprié dans l'en-tête HTTP.

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
//  WebSocket - Envoi des données en temps réel
// ============================================================

// --- Gestionnaire de la connexion WebSocket ---
// Lorsqu'un navigateur ouvre une connexion WebSocket sur /ws,
// Axum appelle ce gestionnaire qui délègue immédiatement à la
// boucle de push. Le mécanisme de mise à niveau (upgrade) est
// géré de manière transparente par la bibliothèque Axum.
async fn handler_websocket(ws: WebSocketUpgrade, State(etat): State<EtatApp>) -> Response {
    ws.on_upgrade(|socket| boucle_push(socket, etat))
}

// --- Boucle de push WebSocket ---
// Cette fonction tourne indéfiniment pour un client donné.
// À chaque itération, elle sérialise l'état courant du tableau
// de bord en JSON et l'envoie via la socket. Si l'envoi échoue
// (client déconnecté), la boucle se termine proprement.
/// Boucle qui envoie l'état du dashboard toutes les secondes
async fn boucle_push(mut socket: WebSocket, etat: EtatApp) {
    loop {
        // Lire l'état courant
        let payload = {
            let dash = etat.dashboard.read().unwrap();
            serde_json::to_string(&*dash).unwrap_or_default()
        };

        // Envoyer au navigateur
        if socket.send(Message::Text(payload)).await.is_err() {
            break; // Client déconnecté
        }

        // Attendre 1 seconde
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ============================================================
//  Tâches asynchrones - tournent en permanence en arrière-plan
// ============================================================

// --- Tâche d'interrogation des agents ---
// Cette tâche s'exécute en boucle infinie et contacte chaque agent
// distant via une requête HTTP GET sur leur point d'accès /api/snapshot.
// Un délai d'attente de 2 secondes est appliqué à chaque requête.
// Si un agent ne répond pas dans ce délai, il est marqué hors ligne
// (`en_ligne: false`) mais reste visible dans le tableau de bord.
// L'état du tableau de bord est mis à jour à l'issue de chaque cycle.
/// Interroge tous les agents configurés et met à jour le dashboard
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

        // Mettre à jour le dashboard
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

// --- Tâche d'évaluation des alertes ---
// À chaque seconde, cette tâche récupère le dernier snapshot local,
// calcule la consommation quota en mégaoctets, puis évalue l'ensemble
// des règles d'alerte définies dans la configuration. Si une ou
// plusieurs règles sont franchies, une notification système est
// émise et les alertes sont ajoutées à la liste du tableau de bord
// (limitée aux 100 dernières). L'état des quotas est également
// recalculé et mis à jour à l'issue de chaque évaluation.
/// Évalue les règles d'alerte et émet les notifications
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

            // Évaluer toutes les règles
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

            // Mettre à jour l'état des quotas
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

// --- Tâche de comptabilisation des quotas ---
// Cette tâche additionne chaque seconde le volume de données
// transitant sur la machine locale (RX + TX en octets/s) au
// compteur cumulé de la machine concernée. Ce compteur sert de
// base au calcul du pourcentage d'utilisation du quota dans la
// tâche d'alertes. Il peut être remis à zéro via l'API REST.
/// Comptabilise les octets consommés par machine pour les quotas
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

// --- API de lecture de la configuration ---
// Retourne la configuration courante sous forme JSON.
// Utilisée par l'interface de paramétrage du tableau de bord.
async fn api_lire_config(State(etat): State<EtatApp>) -> Json<Config> {
    Json(etat.config.read().unwrap().clone())
}

// --- API de sauvegarde de la configuration ---
// Reçoit une nouvelle configuration au format JSON, l'applique
// immédiatement en mémoire et la persiste sur disque via la
// méthode `sauvegarder()`. Retourne un objet JSON indiquant
// le succès ou l'échec de l'opération.
async fn api_sauvegarder_config(
    State(etat): State<EtatApp>,
    Json(nouvelle_config): Json<Config>,
) -> Json<serde_json::Value> {
    let ok = nouvelle_config.sauvegarder().is_ok();
    *etat.config.write().unwrap() = nouvelle_config;
    Json(serde_json::json!({
        "ok":      ok,
        "message": if ok { "Configuration sauvegardée" } else { "Erreur de sauvegarde" }
    }))
}

// --- Corps de la requête de remise à zéro d'un quota ---
// Structure désérialisée depuis le corps JSON de la requête POST.
#[derive(Deserialize)]
struct CorpsResetQuota { machine: String }

// --- API de remise à zéro d'un quota ---
// Supprime l'entrée du compteur d'octets pour la machine spécifiée,
// ce qui remet effectivement son quota à zéro. La prochaine évaluation
// des règles d'alerte reflétera immédiatement cette remise à zéro.
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

// --- Ouverture automatique du navigateur ---
// Lance un thread dédié qui attend 800 ms (le temps que le serveur
// soit prêt) puis ouvre l'URL du tableau de bord dans le navigateur
// par défaut du système. La commande utilisée est adaptée selon
// le système d'exploitation cible (Linux, Windows, macOS).
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

// --- Envoi d'une notification système ---
// Utilise la bibliothèque `notify_rust` pour afficher une notification
// native sur le bureau de l'utilisateur. Chaque notification s'exécute
// dans un thread séparé afin de ne pas bloquer la boucle asynchrone.
// Le délai d'affichage est fixé à 5 secondes.
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

// --- Affichage de la bannière de démarrage ---
// Affiche dans le terminal un encadré récapitulatif indiquant l'URL
// d'accès au tableau de bord et le raccourci clavier d'arrêt.
fn afficher_banniere(port: u16) {
    println!("╔══════════════════════════════════════════╗");
    println!("║       NetWatch — Mode MAÎTRE             ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Dashboard : http://localhost:{:<5}      ║", port);
    println!("║  Ctrl+C pour arrêter                     ║");
    println!("╚══════════════════════════════════════════╝");
}
