# 🌐 NetWatch — Moniteur de Bande Passante Réseau

> **École Nationale Supérieure Polytechnique de Douala**  
> Département Génie Logiciel — GL4 · 2025–2026  
> Matière : Programmation Système avec Rust

---

## 📖 Présentation

NetWatch est un système de surveillance réseau en temps réel développé en **Rust**.  
Il identifie les processus qui consomment le plus de bande passante sur un réseau local,
et les affiche dans un **tableau de bord web** accessible depuis n'importe quel navigateur.

Le système fonctionne selon un modèle **Maître / Agents** :

```
Machine Agent A (192.168.1.101)  ──► netwatch agent  (port 7878)
Machine Agent B (192.168.1.102)  ──► netwatch agent  (port 7878)
                                               │ HTTP toutes les secondes
                                               ▼
Machine Maître  (192.168.1.103)  ──► netwatch master (port 3000)
                                               │ WebSocket
                                               ▼
                                         Navigateur Web
                                      http://localhost:3000
```

---

## ✅ Fonctionnalités

- **Débit réseau en temps réel** — entrant et sortant, par interface
- **Top 20 processus** — triés par consommation CPU/réseau
- **Graphique 60 secondes** — courbe d'évolution du débit
- **Alertes automatiques** — si un processus dépasse le seuil configuré
- **Quotas par machine** — barre de progression + alerte à X%
- **Notifications bureau** — toast système (Linux et Windows)
- **Configuration via l'interface** — sans redémarrer le programme
- **Multi-machines** — surveiller tout un réseau depuis un point central
- **Cross-platform** — Linux et Windows

---

## 🏗️ Structure du projet

```
netwatch/
├── Cargo.toml          Dépendances Rust
├── Cargo.lock          Versions exactes (auto-généré)
├── netwatch.toml       Configuration (ports, agents, quotas, seuils)
├── README.md           Ce fichier
│
├── src/                Code source Rust
│   ├── main.rs         Point d'entrée — lance master ou agent
│   ├── config.rs       Lecture/écriture de netwatch.toml
│   ├── collector.rs    Thread de collecte (sysinfo, toutes les secondes)
│   ├── alerts.rs       Moteur de règles d'alertes (Trait RegleAlerte)
│   ├── agent.rs        Serveur HTTP agent (axum, port 7878)
│   ├── master.rs       Serveur maître (WebSocket, agrégation)
│   └── tests.rs        Tests d'intégration (cargo test)
│
└── static/             Interface web
    ├── index.html      Structure HTML du dashboard (Membre 7)
    ├── style.css       Styles CSS — palette Nexus (Membre 7)
    ├── charts.js       Graphiques Chart.js temps réel (Membre 8)
    ├── settings.js     Formulaires de configuration (Membre 9)
    └── app.js          Logique principale — WebSocket, rendu (Membre 8/9)
```

---

## 🚀 Installation

### Prérequis

- **Rust** ≥ 1.75 → https://rustup.rs
- **Windows uniquement** : Visual Studio Build Tools avec "Desktop C++"

### Compiler

```bash
cd netwatch
cargo build --release
```

Le binaire sera dans `target/release/netwatch` (Linux) ou `target\release\netwatch.exe` (Windows).

---

## ▶️ Utilisation

### Mode Maître — tableau de bord central

```bash
# Linux
./target/release/netwatch master

# Windows
target\release\netwatch.exe master
```

Le navigateur s'ouvre automatiquement sur **http://localhost:3000**

### Mode Agent — machine surveillée

```bash
# Linux
./target/release/netwatch agent

# Windows
target\release\netwatch.exe agent
```

L'agent écoute sur le **port 7878** et attend les requêtes du maître.

### Aide en ligne

```bash
./netwatch --help
./netwatch master --help
```

---

## ⚙️ Configuration

Le fichier `netwatch.toml` se trouve dans le même dossier que l'exécutable.

```toml
# Ports
port_web   = 3000   # Dashboard web (mode maître)
port_agent = 7878   # Écoute des agents

# Machines à surveiller
[[agents]]
nom = "PC-Etudiant-01"
ip  = "192.168.1.101"

[[agents]]
nom = "PC-Etudiant-02"
ip  = "192.168.1.102"

# Quotas de données par machine
[[quotas]]
machine   = "PC-Etudiant-01"
limite_mb = 500

# Seuils d'alerte
[alertes]
debit_max_mbps          = 5.0   # Alerte si un processus > 5 MB/s
avertissement_quota_pct = 80    # Avertissement à 80% du quota
```

> 💡 Vous pouvez aussi tout configurer depuis **l'onglet Réglages** du dashboard — sans toucher au fichier.

---

## 🧪 Lancer les tests

```bash
# Lancer tous les tests
cargo test

# Lancer avec les messages de sortie
cargo test -- --nocapture

# Lancer un test spécifique
cargo test test_cycle_quota_complet
```

Les tests couvrent : la configuration, les règles d'alerte, les quotas, la sérialisation JSON.

---

## 🔌 Pare-feu

Les agents écoutent sur le port **7878**.

```bash
# Linux (UFW)
sudo ufw allow 7878/tcp
sudo ufw allow 3000/tcp

# Windows (PowerShell administrateur)
New-NetFirewallRule -DisplayName "NetWatch Agent" -Direction Inbound -Protocol TCP -LocalPort 7878 -Action Allow
New-NetFirewallRule -DisplayName "NetWatch Master" -Direction Inbound -Protocol TCP -LocalPort 3000 -Action Allow
```

---

## 📡 API REST

| Route | Méthode | Description |
|-------|---------|-------------|
| `/` | GET | Dashboard HTML |
| `/ws` | WebSocket | Push données (1/s) |
| `/api/config` | GET | Lire la configuration |
| `/api/config` | POST | Sauvegarder la configuration |
| `/api/quotas/reset` | POST | Réinitialiser un quota |
| `/api/snapshot` *(agent)* | GET | Dernier snapshot |
| `/api/historique` *(agent)* | GET | 60 derniers snapshots |
| `/api/ping` *(agent)* | GET | Test de connexion |

---

## 🏷️ Concepts Rust illustrés

| Concept | Fichier | Détail |
|---------|---------|--------|
| `Arc<RwLock<T>>` | collector, master | Partage sécurisé entre threads |
| `thread::spawn` | collector | Thread OS natif pour la collecte |
| `mpsc::channel` | agent, master | Signal d'arrêt propre |
| `async fn` / `.await` | master, agent | Handlers HTTP asynchrones |
| `tokio::spawn` | master | Tâches async parallèles |
| `trait RegleAlerte` | alerts | Interface extensible (pattern Trait) |
| `Box<dyn Trait>` | alerts | Dispatch dynamique |
| `serde` Serialize/Deserialize | tous | Sérialisation JSON et TOML |
| `#[cfg(target_os)]` | master | Compilation conditionnelle |
| `include_str!()` | master | Fichiers embarqués dans le binaire |

---

## 👥 Équipe

| Membre | Branche | Fichier | Contribution |
|--------|---------|---------|--------------|
| Membre 1 | `main` | Tous | Chef de projet, intégration |
| Membre 2 | `feature/config` | `src/config.rs` | Configuration TOML |
| Membre 3 | `feature/collector` | `src/collector.rs` | Collecte sysinfo |
| Membre 4 | `feature/alerts` | `src/alerts.rs` | Moteur d'alertes |
| Membre 5 | `feature/agent` | `src/agent.rs` | Serveur HTTP agent |
| Membre 6 | `feature/master` | `src/master.rs` | Serveur maître |
| Membre 7 | `feature/dashboard-layout` | `static/index.html` + `style.css` | HTML/CSS |
| Membre 8 | `feature/dashboard-charts` | `static/charts.js` | Graphiques |
| Membre 9 | `feature/dashboard-settings` | `static/settings.js` | Formulaires |
| Membre 10 | `feature/tests` | `src/tests.rs` | Tests unitaires |

---

> **Licence** : Usage académique — ENSPD GL4 · 2025–2026
