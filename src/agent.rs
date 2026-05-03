// ============================================================
//  Membre 5 - Serveur HTTP Agent
//
//  Ce module lance NetWatch en mode "agent" :
//  il collecte les statistiques locales et les expose
//  via une API HTTP que le serveur maitre peut interroger.
//
//  Routes disponibles :
//    GET /api/snapshot  -> dernier instantane (JSON)
//    GET /api/historique-> 60 derniers instantanes (JSON)
//    GET /api/ping      -> test de connexion
// ============================================================

use crate::collector;
use crate::collector::{SharedStore, Snapshot};
use crate::config::Config;
use axum::{extract::State, routing::get, Json, Router};
use std::sync::mpsc;
use tokio::net::TcpListener;

// ============================================================
//  Point d entree du mode agent
// ============================================================

/// Lance l agent : collecte locale + serveur HTTP
pub async fn lancer() {
    // Charger la configuration
    let config = Config::charger();
    let port = config.port_agent;

    // Creer le store partage (historique des snapshots)
    let store = collector::nouveau_store();

    // Canal pour arreter proprement le thread de collecte
    let (signal_arret_tx, signal_arret_rx) = mpsc::channel::<()>();

    // Demarrer la collecte en arriere-plan
    let _thread_collecte = collector::demarrer_collecte(store.clone(), signal_arret_rx);

    // Afficher la banniere de demarrage
    afficher_banniere(port);

    // Definir les routes HTTP
    let app = Router::new()
        .route("/api/snapshot",   get(route_snapshot))
        .route("/api/historique", get(route_historique))
        .route("/api/ping",       get(route_ping))
        .with_state(store); // Injecter le store dans tous les handlers

    // Demarrer le serveur
    let adresse = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&adresse).await
        .unwrap_or_else(|_| panic!("Impossible d ecouter sur le port {}", port));

    println!("[agent] Serveur demarre sur {}", adresse);

    // Attendre une interruption (Ctrl+C) ou une erreur serveur
    tokio::select! {
        // Si le serveur HTTP s arrete
        resultat = axum::serve(listener, app) => {
            if let Err(e) = resultat {
                eprintln!("[agent] Erreur serveur : {}", e);
            }
        }
        // Si l utilisateur appuie sur Ctrl+C
        _ = tokio::signal::ctrl_c() => {
            println!("\\n[agent] Arret demande...");
            // Envoyer le signal d arret au thread de collecte
            signal_arret_tx.send(()).ok();
            println!("[agent] Arret propre effectue.");
        }
    }
}

// ============================================================
//  Handlers HTTP
//  Chaque fonction est appellee quand une route est demandee
// ============================================================

/// GET /api/snapshot -> retourne le dernier snapshot
/// Option<Snapshot> = Some(snapshot) ou None si pas encore collecte
async fn route_snapshot(State(store): State<SharedStore>) -> Json<Option<Snapshot>> {
    let donnees = store.read().unwrap();
    Json(donnees.back().cloned())
}

/// GET /api/historique -> retourne les 60 derniers snapshots
async fn route_historique(State(store): State<SharedStore>) -> Json<Vec<Snapshot>> {
    let donnees = store.read().unwrap();
    Json(donnees.iter().cloned().collect())
}

/// GET /api/ping -> simple test de connectivite
async fn route_ping() -> &'static str {
    "pong"
}

// ============================================================
//  Utilitaire
// ============================================================

fn afficher_banniere(port: u16) {
    println!("╔══════════════════════════════════════════╗");
    println!("║       NetWatch — Mode AGENT              ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Ecoute sur le port {:>5}               ║", port);
    println!("║  Ctrl+C pour arreter                     ║");
    println!("╚══════════════════════════════════════════╝");
}

