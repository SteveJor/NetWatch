# 🛠️ NetWatch — Guide d'installation et d'exécution

> Ce guide couvre **deux scénarios** :
> - **Scénario A** — Vous avez le **binaire pré-compilé** (`.exe` Windows ou exécutable Linux) → allez directement à la [Partie 3](#partie-3--exécution-avec-le-binaire-pré-compilé)
> - **Scénario B** — Vous avez le **code source** et devez compiler → lisez tout depuis le début

---

## Table des matières

1. [Prérequis système](#1-prérequis-système)
2. [Installation de Rust (compilation depuis les sources)](#2-installation-de-rust-compilation-depuis-les-sources)
3. [Partie 3 : Exécution avec le binaire pré-compilé](#partie-3--exécution-avec-le-binaire-pré-compilé)
4. [Compilation depuis les sources](#4-compilation-depuis-les-sources)
5. [Lancement du programme](#5-lancement-du-programme)
6. [Configuration initiale](#6-configuration-initiale)
7. [Déploiement en réseau (multi-machines)](#7-déploiement-en-réseau-multi-machines)
8. [Pare-feu](#8-pare-feu)
9. [Vérification que tout fonctionne](#9-vérification-que-tout-fonctionne)
10. [Dépannage](#10-dépannage)

---

## 1. Prérequis système

### Systèmes d'exploitation supportés

| OS | Version minimale | Architecture |
|----|-----------------|--------------|
| Windows | Windows 10 / Windows Server 2019 | x86_64 |
| Linux | Ubuntu 20.04+ / Debian 11+ / toute distro avec glibc ≥ 2.31 | x86_64 |

### Logiciels requis (si vous compilez depuis les sources)

| Logiciel | Version minimale | Obligatoire |
|----------|-----------------|-------------|
| Rust | 1.75.0 | ✅ Oui |
| Git | 2.x | ✅ Oui (pour cloner) |
| Visual Studio Build Tools *(Windows seulement)* | 2017 ou plus récent | ✅ Oui sur Windows |

### Logiciels requis (si vous utilisez le binaire pré-compilé)

Aucune dépendance à installer — le binaire est **statiquement lié** et s'exécute seul.

> ⚠️ Exception Linux : si votre système est très ancien, vous pourriez avoir besoin de mettre à jour `glibc`. Sur Ubuntu 20.04+ et Debian 11+, c'est déjà le cas.

---

## 2. Installation de Rust (compilation depuis les sources)

> **Ignorez cette section si vous avez déjà le binaire pré-compilé.**

### Sur Linux / macOS

```bash
# Télécharger et exécuter l'installateur officiel
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Suivre les instructions à l'écran (appuyer sur 1 pour installation par défaut)
# Puis recharger l'environnement :
source $HOME/.cargo/env

# Vérifier l'installation
rustc --version
# Doit afficher : rustc 1.75.0 (ou plus récent)

cargo --version
# Doit afficher : cargo 1.75.0 (ou plus récent)
```

---

### Sur Windows

#### Étape 1 — Installer Visual Studio Build Tools (OBLIGATOIRE)

Rust sur Windows utilise le compilateur C++ de Microsoft pour lier les binaires. Il y a 3 cas :

---

##### ✅ Cas 1 — Vous n'avez rien d'installé (installation fraîche)

1. Aller sur : **https://visualstudio.microsoft.com/visual-cpp-build-tools/**
2. Télécharger **"Build Tools for Visual Studio 2022"**
3. Lancer l'installateur
4. Dans la liste des composants, cocher **"Développement Desktop en C++"**

   > ⚠️ **VS Code est un éditeur de texte, ce n'est PAS Visual Studio. Il ne suffit pas.**

5. Cliquer **Installer** (~6 GB, environ 10–15 minutes selon la connexion)
6. **Redémarrer le PC** après installation

---

##### ✅ Cas 2 — Vous avez déjà Visual Studio installé (2017 ou plus récent)

Il faut juste ajouter la composante C++ si elle n'est pas déjà installée :

1. Ouvrir **Visual Studio Installer** (chercher dans le menu Démarrer)
2. Cliquer **"Modifier"** sur votre installation Visual Studio
3. Dans l'onglet **"Charges de travail"**, cocher **"Développement Desktop en C++"**
4. Cliquer **"Modifier"** (en bas à droite)
5. Attendre la fin de l'installation
6. **Redémarrer le PC**

---

##### ✅ Cas 3 — Alternative sans Visual Studio (GNU toolchain)

Si vous ne voulez pas installer Visual Studio, utilisez le toolchain GNU (MinGW) :

```cmd
rustup target add x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

> ⚠️ Le toolchain GNU fonctionne mais peut avoir des limitations avec certaines bibliothèques système Windows. Le toolchain MSVC (Cas 1 ou 2) est recommandé pour un projet sérieux.

---

#### Étape 2 — Installer Rust sur Windows

1. Aller sur **https://rustup.rs**
2. Cliquer **"Download rustup-init.exe (64-bit)"**
3. Lancer `rustup-init.exe`
4. Appuyer sur **`1`** (installation par défaut), puis **Entrée**
5. **Fermer et rouvrir** le terminal (PowerShell ou cmd)
6. Vérifier :

```cmd
rustc --version
cargo --version
```

---

## Partie 3 : Exécution avec le binaire pré-compilé

> Cette section s'adresse à ceux qui ont **directement le fichier `.exe`** (Windows) ou l'**exécutable Linux** sans avoir besoin de compiler.

### Sur Windows (fichier `netwatch.exe`)

#### Étape 1 — Préparer le dossier

Créer un dossier dédié et y placer le fichier :
```
C:\netwatch\
    └── netwatch.exe
```

Créer le fichier de configuration `netwatch.toml` dans le **même dossier** :
```
C:\netwatch\
    ├── netwatch.exe
    └── netwatch.toml      ← créer ce fichier (voir section 6)
```

#### Étape 2 — Ouvrir un terminal dans ce dossier

- Ouvrir l'**Explorateur de fichiers**
- Naviguer vers `C:\netwatch\`
- Dans la barre d'adresse, taper `cmd` et appuyer sur Entrée

#### Étape 3 — Lancer en mode maître

```cmd
netwatch.exe master
```

Le navigateur s'ouvre automatiquement sur `http://localhost:3000`

#### Étape 4 — Lancer en mode agent (sur d'autres machines)

Copier `netwatch.exe` et `netwatch.toml` sur la machine agent, puis :

```cmd
netwatch.exe agent
```

---

### Sur Linux (fichier `netwatch`)

#### Étape 1 — Rendre le fichier exécutable

```bash
# Naviguer vers le dossier contenant le fichier
cd ~/Bureau/netwatch/      # ou le chemin vers votre dossier

# Rendre exécutable
chmod +x netwatch

# Vérifier
ls -la netwatch
# Doit afficher : -rwxr-xr-x ... netwatch
```

#### Étape 2 — Créer le fichier de configuration

```bash
# Créer netwatch.toml dans le même dossier (voir section 6)
nano netwatch.toml
```

#### Étape 3 — Lancer

```bash
# Mode maître
./netwatch master

# Mode agent (sur une autre machine)
./netwatch agent
```

> **Si Linux dit "permission denied" :** `chmod +x ./netwatch`  
> **Si Linux dit "no such file or directory" mais que le fichier existe :** le binaire a peut-être été compilé pour une autre architecture. Recompilez depuis les sources.

---

## 4. Compilation depuis les sources

### Étape 1 — Obtenir le code source

```bash
# Option A : cloner depuis GitHub
git clone https://github.com/TON_USERNAME/netwatch.git
cd netwatch

# Option B : extraire depuis le ZIP
# (décompresser netwatch_projet_complet.zip puis :)
cd netwatch
```

### Étape 2 — Compiler

```bash
# Linux / macOS
cargo build --release

# Windows (cmd ou PowerShell)
cargo build --release
```

> La première compilation télécharge toutes les dépendances depuis **crates.io**.  
> Durée : **2 à 5 minutes** selon la connexion internet et la puissance de la machine.  
> Les compilations suivantes sont beaucoup plus rapides (cache Cargo).

### Où se trouve le binaire compilé ?

| OS | Chemin |
|----|--------|
| Linux | `target/release/netwatch` |
| Windows | `target\release\netwatch.exe` |

### Étape 3 — Vérifier la compilation

```bash
# Linux
./target/release/netwatch --help

# Windows
target\release\netwatch.exe --help
```

Doit afficher :
```
NetWatch — Moniteur réseau temps réel (ENSPD GL4)

Usage: netwatch <COMMAND>

Commands:
  master  Lance le tableau de bord maître
  agent   Lance l'agent de collecte
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

---

## 5. Lancement du programme

### Mode Maître (tableau de bord centralisé)

À lancer sur **une seule machine** — celle qui aura le dashboard.

```bash
# Linux
./target/release/netwatch master
# ou (si binaire pré-compilé)
./netwatch master

# Windows
target\release\netwatch.exe master
# ou
netwatch.exe master
```

**Ce qui se passe au démarrage :**
1. Le fichier `netwatch.toml` est lu (ou créé avec les valeurs par défaut)
2. Le thread de collecte locale démarre
3. Le serveur web démarre sur `http://0.0.0.0:3000`
4. Le navigateur par défaut s'ouvre sur `http://localhost:3000`
5. Les agents configurés sont interrogés toutes les secondes

**Sortie attendue dans le terminal :**
```
╔══════════════════════════════════════════╗
║       NetWatch — Mode MAÎTRE             ║
╠══════════════════════════════════════════╣
║  Dashboard → http://localhost:3000       ║
║  Ctrl+C pour arrêter proprement          ║
╚══════════════════════════════════════════╝
```

---

### Mode Agent (collecte sur une machine distante)

À lancer sur **chaque machine à surveiller** (sauf la machine maître, qui se surveille elle-même).

```bash
# Linux
./netwatch agent

# Windows
netwatch.exe agent
```

**Sortie attendue dans le terminal :**
```
╔══════════════════════════════════════════╗
║       NetWatch — Mode AGENT              ║
╠══════════════════════════════════════════╣
║  Écoute sur le port  7878                ║
║  Ctrl+C pour arrêter proprement          ║
╚══════════════════════════════════════════╝
```

### Arrêt propre

```bash
# Dans le terminal où NetWatch tourne :
Ctrl + C
```

NetWatch arrête proprement le thread de collecte avant de quitter.

---

## 6. Configuration initiale

### Créer / éditer `netwatch.toml`

Le fichier `netwatch.toml` doit être dans le **même dossier que l'exécutable**.

**Contenu minimal (maître sans agents) :**
```toml
web_port   = 3000
agent_port = 7878

[alert_rules]
max_rate_mbps      = 5.0
quota_warn_percent = 80
suspicious_ips     = []
```

**Contenu complet (avec agents et quotas) :**
```toml
web_port   = 3000
agent_port = 7878

[[agents]]
name = "PC-Etudiant-01"
ip   = "192.168.1.101"

[[agents]]
name = "PC-Etudiant-02"
ip   = "192.168.1.102"

[[quotas]]
machine  = "PC-Etudiant-01"
limit_mb = 500

[[quotas]]
machine  = "PC-Etudiant-02"
limit_mb = 300

[alert_rules]
max_rate_mbps      = 5.0
quota_warn_percent = 80
suspicious_ips     = []
```

> 💡 **Astuce :** Vous pouvez aussi tout configurer depuis l'interface web (onglet **Réglages**) — le fichier `netwatch.toml` est automatiquement mis à jour.

### Trouver l'adresse IP d'une machine

**Linux :**
```bash
ip a
# ou
hostname -I
```

**Windows :**
```cmd
ipconfig
# Chercher "Adresse IPv4" sous l'interface active (Ethernet ou Wi-Fi)
```

---

## 7. Déploiement en réseau (multi-machines)

### Schéma type (3 machines)

```
Machine A (192.168.1.101) → netwatch.exe agent
Machine B (192.168.1.102) → netwatch.exe agent
Machine C (192.168.1.103) → netwatch.exe master  (avec netwatch.toml configuré)
```

### Étapes

**1. Sur chaque machine agent (A et B) :**
```bash
# Copier netwatch (ou netwatch.exe) sur la machine
# Créer netwatch.toml minimal :
cat > netwatch.toml << 'EOF'
web_port   = 3000
agent_port = 7878
[alert_rules]
max_rate_mbps = 5.0
quota_warn_percent = 80
suspicious_ips = []
EOF

# Lancer l'agent
./netwatch agent
```

**2. Sur la machine maître (C) :**

Éditer `netwatch.toml` pour déclarer les agents :
```toml
web_port   = 3000
agent_port = 7878

[[agents]]
name = "Machine-A"
ip   = "192.168.1.101"

[[agents]]
name = "Machine-B"
ip   = "192.168.1.102"

[alert_rules]
max_rate_mbps      = 5.0
quota_warn_percent = 80
suspicious_ips     = []
```

Puis lancer :
```bash
./netwatch master
```

**3. Vérifier la connexion aux agents depuis le dashboard :**
- Aller sur `http://localhost:3000`
- Dans la sidebar, les agents doivent apparaître avec un **point vert** (en ligne)
- Un **point gris** = agent hors ligne ou pare-feu bloquant

---

## 8. Pare-feu

Les agents écoutent sur le port **7878**. Ce port doit être ouvert sur les machines agents.

### Linux (UFW)

```bash
# Autoriser le port agent
sudo ufw allow 7878/tcp

# Autoriser le port maître (si accès depuis d'autres machines)
sudo ufw allow 3000/tcp

# Vérifier
sudo ufw status
```

### Linux (firewalld — CentOS/Fedora/RHEL)

```bash
sudo firewall-cmd --permanent --add-port=7878/tcp
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload
```

### Windows (Pare-feu Windows)

**Méthode graphique :**
1. Ouvrir **Pare-feu Windows Defender avec sécurité avancée**
2. **Règles de trafic entrant** → **Nouvelle règle**
3. Type : **Port** → TCP → Port spécifique : **7878**
4. Action : **Autoriser la connexion**
5. Profils : cocher **Domaine**, **Privé**, **Public**
6. Nom : `NetWatch Agent`
7. **Terminer**

**Méthode ligne de commande (PowerShell administrateur) :**
```powershell
# Ouvrir PowerShell en tant qu'administrateur, puis :

# Port agent
New-NetFirewallRule -DisplayName "NetWatch Agent" -Direction Inbound `
    -Protocol TCP -LocalPort 7878 -Action Allow

# Port maître (optionnel)
New-NetFirewallRule -DisplayName "NetWatch Master" -Direction Inbound `
    -Protocol TCP -LocalPort 3000 -Action Allow
```

---

## 9. Vérification que tout fonctionne

### Test 1 — Vérifier que le maître démarre

```bash
./netwatch master
```
✅ Le navigateur s'ouvre sur `http://localhost:3000`  
✅ Les cartes "Réception" et "Envoi" affichent des valeurs  
✅ Le graphique se remplit seconde par seconde  
✅ Le tableau "Top processus" liste les programmes actifs  

### Test 2 — Vérifier qu'un agent est joignable

Depuis la machine maître, tester la connexion à un agent :

```bash
# Linux
curl http://192.168.1.101:7878/api/ping
# Doit retourner : pong

# Windows (PowerShell)
Invoke-WebRequest http://192.168.1.101:7878/api/ping
# Doit retourner : StatusCode 200, Content: pong
```

### Test 3 — Vérifier les alertes

Générer du trafic réseau artificiel pour déclencher une alerte :

```bash
# Linux — télécharger un gros fichier
wget -O /dev/null http://speedtest.tele2.net/100MB.zip

# Windows — télécharger un fichier test
# Ouvrir un navigateur et lancer un speedtest sur fast.com
```

Si le débit dépasse le seuil configuré :
- ✅ Une alerte apparaît dans l'onglet **Alertes**
- ✅ Une **notification bureau** (toast) apparaît
- ✅ Le badge rouge sur l'onglet Alertes s'incrémente

### Test 4 — Vérifier les quotas

1. Dans l'onglet **Réglages**, ajouter un quota de **10 MB** sur la machine locale
2. Cliquer **Sauvegarder**
3. Générer du trafic (naviguer sur internet, télécharger un fichier)
4. ✅ La barre de progression du quota augmente
5. À 80% → ✅ Alerte "Quota ⚠️"
6. À 100% → ✅ Alerte "Quota dépassé 🚫"
7. Cliquer **Réinitialiser** → ✅ La barre revient à 0%

---

## 10. Dépannage

### ❌ `error: linker link.exe not found` (Windows)

**Cause :** Visual Studio Build Tools avec le composant C++ n'est pas installé.  
**Solution :** Voir [Section 2 — Cas 1 ou Cas 2](#-cas-1--vous-navez-rien-dinstallé-installation-fraîche)

---

### ❌ `Address already in use (os error 98)`

**Cause :** Le port 3000 (maître) ou 7878 (agent) est déjà utilisé par un autre programme.

**Linux :**
```bash
# Trouver quel programme utilise le port
sudo lsof -i :3000
sudo lsof -i :7878

# Tuer le processus (remplacer PID par le numéro)
kill -9 PID
```

**Windows :**
```cmd
netstat -ano | findstr :3000
taskkill /PID <PID> /F
```

**Alternative :** changer le port dans `netwatch.toml` :
```toml
web_port   = 3001   # changer ici
agent_port = 7879   # changer ici
```

---

### ❌ Le navigateur ne s'ouvre pas automatiquement

**Cause :** `xdg-open` absent sur Linux, ou le délai est trop court.

**Solution :** Ouvrir manuellement `http://localhost:3000` dans le navigateur.

**Linux — installer xdg-utils si absent :**
```bash
sudo apt install xdg-utils   # Debian/Ubuntu
```

---

### ❌ Un agent apparaît hors ligne (point gris)

**Vérifications dans l'ordre :**

1. L'agent tourne-t-il sur la machine distante ?
   ```bash
   # Depuis la machine agent
   ./netwatch agent
   # La bannière doit apparaître
   ```

2. L'IP est-elle correcte dans `netwatch.toml` ?
   ```bash
   # Sur la machine agent
   hostname -I    # Linux
   ipconfig       # Windows
   ```

3. Le port 7878 est-il ouvert ? (voir [Section 8](#8-pare-feu))

4. Test direct depuis la machine maître :
   ```bash
   curl http://IP_AGENT:7878/api/ping
   ```

---

### ❌ `permission denied` sur Linux

```bash
chmod +x ./netwatch
./netwatch master
```

---

### ❌ Notifications bureau absentes sur Linux

```bash
# Installer libnotify
sudo apt install libnotify-bin    # Debian/Ubuntu
sudo dnf install libnotify        # Fedora
```

---

### ❌ `cargo build` échoue avec des erreurs réseau

**Cause :** Pas de connexion internet pour télécharger les crates.  
**Solution :** Se connecter à internet, puis réessayer. Cargo met en cache les dépendances dans `~/.cargo/registry/` — les compilations suivantes n'ont plus besoin d'internet.

---

### ❌ `could not compile proc-macro2` ou `quote`

**Cause :** Même erreur que `link.exe not found` — Build Tools manquants.  
**Solution :** [Section 2 — Windows](#sur-windows)

---

## Résumé des commandes essentielles

```bash
# ── Compilation ────────────────────────────────────
cargo build --release          # compiler le projet

# ── Lancement ──────────────────────────────────────
./netwatch master              # Linux — mode maître
./netwatch agent               # Linux — mode agent
netwatch.exe master            # Windows — mode maître
netwatch.exe agent             # Windows — mode agent

# ── Aide ───────────────────────────────────────────
./netwatch --help              # liste des commandes
./netwatch --version           # version

# ── Test agent depuis le maître ────────────────────
curl http://IP_AGENT:7878/api/ping     # doit répondre "pong"
curl http://IP_AGENT:7878/api/snapshot # snapshot JSON

# ── Dashboard ──────────────────────────────────────
# Ouvrir dans le navigateur :
http://localhost:3000
```