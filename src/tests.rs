use crate::alerts::{RegleDebit, RegleQuota, RegleAlerte};
use crate::collector::Snapshot;
use crate::config::{Config, ReglesAlerte};

fn creer_snapshot(machine: &str, rx: f64, tx: f64) -> Snapshot {
    Snapshot {
        horodatage:   12345,
        machine:      machine.to_string(),
        processus:    Vec::new(),
        interfaces:   Vec::new(),
        total_rx_bps: rx,
        total_tx_bps: tx,
    }
}

#[test]
fn test_config_et_alertes_coherents() {
    let config = Config::default();
    let regle = RegleDebit { seuil_mbps: config.alertes.debit_max_mbps };
    assert_eq!(regle.seuil_mbps, 5.0,
        "La regle de debit doit utiliser le seuil de la config");
}

#[test]
fn test_scenario_complet_alerte_debit() {
    let regle = RegleDebit { seuil_mbps: 1.0 };
    let mut snap = creer_snapshot("PC-Test", 2_000_000.0, 500_000.0);

    snap.processus.push(crate::collector::StatProcessus {
        pid:     123,
        nom:     "chrome".to_string(),
        cpu_pct: 100.0,
        ram_mb:  500.0,
        rx_bps:  2_000_000.0,
        tx_bps:  500_000.0,
    });

    let alertes = regle.evaluer(&snap, 0);
    assert!(!alertes.is_empty(), "Une alerte doit etre generee pour 2 MB/s avec un seuil de 1 MB/s");
    assert!(alertes[0].message.contains("chrome"),
        "Le message d alerte doit contenir le nom du processus");
}

#[test]
fn test_cycle_quota_complet() {
    let machine = "PC-Etudiant-01";
    let regle = RegleQuota {
        machine:    machine.to_string(),
        limite_mb:  100,
        seuil_pct:  80,
    };
    let snap = creer_snapshot(machine, 0.0, 0.0);

    let alertes = regle.evaluer(&snap, 50);
    assert!(alertes.is_empty(), "Pas d alerte a 50% du quota");

    let alertes = regle.evaluer(&snap, 85);
    assert_eq!(alertes.len(), 1, "Un avertissement a 85%");
    assert!(alertes[0].message.contains("85"),
        "Le message doit contenir le pourcentage");

    let alertes = regle.evaluer(&snap, 110);
    assert_eq!(alertes.len(), 1, "Une alerte quota depasse");
    assert!(alertes[0].message.contains("depasse"),
        "Le message doit indiquer que le quota est depasse");
}

#[test]
fn test_isolation_par_machine() {
    let regle = RegleQuota {
        machine:    "Machine-A".to_string(),
        limite_mb:  100,
        seuil_pct:  80,
    };

    let snap = creer_snapshot("Machine-B", 0.0, 0.0);
    let alertes = regle.evaluer(&snap, 200);

    assert!(alertes.is_empty(),
        "Pas d alerte pour une machine non concernee par cette regle");
}

#[test]
fn test_plusieurs_regles() {
    let regles: Vec<Box<dyn RegleAlerte>> = vec![
        Box::new(RegleDebit { seuil_mbps: 1.0 }),
        Box::new(RegleQuota {
            machine:    "PC-Test".to_string(),
            limite_mb:  100,
            seuil_pct:  80,
        }),
    ];

    let mut snap = creer_snapshot("PC-Test", 2_000_000.0, 0.0);
    snap.processus.push(crate::collector::StatProcessus {
        pid: 1, nom: "test".to_string(),
        cpu_pct: 100.0, ram_mb: 100.0,
        rx_bps: 2_000_000.0, tx_bps: 0.0,
    });

    let mut toutes_alertes = Vec::new();
    for regle in &regles {
        toutes_alertes.extend(regle.evaluer(&snap, 90));
    }

    assert!(toutes_alertes.len() >= 2,
        "Au moins 2 alertes attendues (debit + quota)");
}

#[test]
fn test_serialisation_config_complete() {
    let config_originale = Config {
        port_web:   8080,
        port_agent: 9090,
        agents: vec![crate::config::AgentConfig {
            nom: "PC-01".to_string(),
            ip:  "192.168.1.1".to_string(),
        }],
        quotas: vec![crate::config::QuotaConfig {
            machine:    "PC-01".to_string(),
            limite_mb:  500,
        }],
        alertes: ReglesAlerte {
            debit_max_mbps:          10.0,
            avertissement_quota_pct: 75,
        },
    };

    let json = serde_json::to_string(&config_originale).expect("Serialisation JSON echouee");
    let config_restauree: Config = serde_json::from_str(&json)
        .expect("Deserialisation JSON echouee");

    assert_eq!(config_restauree.port_web, 8080);
    assert_eq!(config_restauree.port_agent, 9090);
    assert_eq!(config_restauree.agents.len(), 1);
    assert_eq!(config_restauree.agents[0].nom, "PC-01");
    assert_eq!(config_restauree.quotas[0].limite_mb, 500);
    assert_eq!(config_restauree.alertes.debit_max_mbps, 10.0);
}