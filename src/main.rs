
mod config;    // Lecture et ecriture de la configuration
mod collector; // Collecte des statistiques reseau
mod alerts;    // Moteur de regles d alertes
mod agent;     // Mode agent (machine surveillee)
mod master;    // Mode maitre (tableau de bord)

// Tests supplementaires (Membre 10)
#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name    = "netwatch",
    about   = "Moniteur de bande passante reseau en temps reel - ENSPD GL4",
    version = "1.0.0"
)]
struct Arguments {
    #[command(subcommand)]
    mode: Mode,
}

/// Les deux modes de lancement
#[derive(Subcommand)]
enum Mode {
    /// Lance le serveur maitre avec le tableau de bord web
    Master,
    /// Lance l agent de surveillance sur cette machine
    Agent,
}

/// Fonction principale - point d entree du programme
/// #[tokio::main] est necessaire pour utiliser async/await dans main()
#[tokio::main]
async fn main() {
    // Lire les arguments de la ligne de commande
    let args = Arguments::parse();

    // Lancer le bon mode
    match args.mode {
        Mode::Master => master::lancer().await,
        Mode::Agent  => agent::lancer().await,
    }
}
