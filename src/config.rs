//Ce fichier definit la structure de la configuration de Netwatch et permet de lire / ecrire le fichier netwatch.toml

use serde::{Deserialize, Serialize};
use std::fs;

//Chemein du fichier de configuration (on pourra le changer si besoin)
const FICHIER_CONFIG: &str = "netwatch.toml";

//La configuration contient tout ce que l'utilisateur peut régler
//Les champs avec #[serde(default)] ont une valeur par défaut si la ligne est absente du fichier de configuration

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    //Le tableau de bord web tourne par défaut sur le port 3000
    #[serde(default = "port_web_defaut")]
    pub port_web: u16,

    //Les agents écoutent sur le port 7878 par défaut
    #[serde(default = "port_agent_defaut")]
    pub port_agent: u16,

    //Liste des machines agents à surveiller
    #[serde(default)]
    pub agents: Vec<AgentConfig>,

    //Les quotas réseau par machine
    #[serde(default)]
    pub quotas: Vec<QuotaConfig>,

    //Les seuils qui déclenchent les alertes
    #[serde(default)]
    pub alertes: ReglesAlerte,
}

// Une machine agent est identifiée juste son nom et son adresse IP dans le réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub nom: String,
    pub ip: String,
}

// Le quota d'une machine est identifié par son nom et la limite en Mo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub machine: String,
    pub limite_mb: u64,
}

// Les seuils à partir desquels on déclenche une alerte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReglesAlerte {
    // Alerte si un processus dépasse ce debit en MB/s
    #[serde(default = "debit_max_defaut")]
    pub debit_max_mbps: f64,

    // Pourcentage de quota pour un avertissement
    #[serde(default = "avertissement_quota_defaut")]
    pub avertissement_quota_pct: u64,
}


//Valeurs par défaut si un champ est absent du fichier

fn port_web_defaut()            -> u16 { 3000 }
fn port_agent_defaut()          -> u16 { 7878 }
fn debit_max_defaut()           -> f64 { 5.0  }
fn avertissement_quota_defaut() -> u64 { 80   }

impl Default for ReglesAlerte {
    fn default() -> Self {
        Self {
            debit_max_mbps:          5.0,
            avertissement_quota_pct: 80,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port_web:   3000,
            port_agent: 7878,
            agents:     Vec::new(),
            quotas:     Vec::new(),
            alertes:    ReglesAlerte::default(),
        }
    }
}

//  Lecture et écriture du fichier

impl Config {
    // Charge la configuration depuis le fichier netwatch.toml
    // Si le fichier n'existe pas ou est cassé, on retourne les valeurs par défaut
    pub fn charger() -> Self {
        match fs::read_to_string(FICHIER_CONFIG) {
            Ok(contenu) => {
                // Si le fichier est trouvé, on essaie de le parser
                toml::from_str(&contenu).unwrap_or_else(|erreur| {
                    eprintln!("[config] Fichier invalide : {}", erreur);
                    eprintln!("[config] On repart sur les valeurs par defaut.");
                    Config::default()
                })
            }
            Err(_) => {
                // Si le fichier n'est pas trouvé, on utilise les valeurs par défauts
                println!("[config] {} absent, valeurs par defaut utilisees.", FICHIER_CONFIG);
                Config::default()
            }
        }
    }

    // Sauvegarde de la configuration dans netwatch.toml
    // Retourne un message d'erreur si l'écriture échoue
    pub fn sauvegarder(&self) -> Result<(), String> {
        // Convertir la configuration en texte TOML lisible
        let contenu = toml::to_string_pretty(self)
            .map_err(|e| format!("Conversion TOML impossible : {}", e))?;

        // Écrire dans le fichier
        fs::write(FICHIER_CONFIG, contenu)
            .map_err(|e| format!("Ecriture du fichier impossible : {}", e))
    }
}


//  Tests unitaires (cargo tests)

#[cfg(test)]
mod tests {
    use super::*;

    // Les valeurs par défaut doivent être exactement celles-là
    #[test]
    fn test_config_valeurs_defaut() {
        let config = Config::default();
        assert_eq!(config.port_web,   3000, "Le port web par defaut doit etre 3000");
        assert_eq!(config.port_agent, 7878, "Le port agent par defaut doit etre 7878");
        assert!(config.agents.is_empty(), "La liste des agents doit etre vide par defaut");
        assert!(config.quotas.is_empty(), "La liste des quotas doit etre vide par defaut");
    }

    // Les seuils d'alerte par défaut
    #[test]
    fn test_regles_alerte_defaut() {
        let regles = ReglesAlerte::default();
        assert_eq!(regles.debit_max_mbps, 5.0,
            "Le seuil de debit par defaut doit etre 5.0 MB/s");
        assert_eq!(regles.avertissement_quota_pct, 80,
            "Le seuil d avertissement quota doit etre 80%");
    }

    // On vérifie qu'on peut bien convertir la configuration en TOML sans erreur (la sérialisation fonctionne)
    #[test]
    fn test_serialisation_toml() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config);
        assert!(toml_str.is_ok(), "La config doit pouvoir etre convertie en TOML");
    }

    // On vérifie que meme sans fichier .toml le chargement plante pas
    #[test]
    fn test_chargement_sans_fichier() {
        let config = Config::charger();
        assert!(config.port_web > 0, "Le port doit etre positif");
    }
}