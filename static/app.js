
/** Donnees recues via WebSocket (mis a jour chaque seconde) */
window.donneesWS = null;

/** Configuration actuelle chargee depuis /api/config */
window.configActuelle = null;

/** Machine selectionnee dans la sidebar ("local" ou nom de l agent) */
let machineSelectionnee = 'local';

/** Compteur de nouvelles alertes (pour le badge rouge) */
let nbNouvellesAlertes = 0;

/** Ensemble des alertes deja vues (evite de compter en double) */
const alertesVues = new Set();

/** Instance du graphique Chart.js (cree par charts.js) */
let graphiqueDebit = null;

// ============================================================
// Connexion WebSocket
// ============================================================

/**
 * Etablit la connexion WebSocket avec le serveur.
 * En cas de deconnexion, tente de se reconnecter apres 2 secondes.
 */
function connecterWebSocket() {
  const ws = new WebSocket('ws://' + location.host + '/ws');

  ws.onopen = function() {
    // Connexion etablie : mettre le point en vert
    document.getElementById('ws-point').className  = 'ws-point ok';
    document.getElementById('ws-statut').textContent = 'En direct';
  };

  ws.onclose = function() {
    // Connexion perdue : mettre le point en rouge et retenter
    document.getElementById('ws-point').className  = 'ws-point erreur';
    document.getElementById('ws-statut').textContent = 'Reconnexion...';
    setTimeout(connecterWebSocket, 2000);
  };

  ws.onerror = function() {
    document.getElementById('ws-statut').textContent = 'Erreur WS';
  };

  ws.onmessage = function(evenement) {
    // Decoder le JSON recu
    try {
      window.donneesWS = JSON.parse(evenement.data);
    } catch (e) {
      return; // Ignorer les messages invalides
    }

    // Mettre a jour l heure de la derniere mise a jour
    const maintenant = new Date().toLocaleTimeString('fr-FR');
    document.getElementById('heure-maj').textContent = 'Maj : ' + maintenant;

    // Redessiner tout le dashboard
    rendreDashboard();
  };
}

// ============================================================
// Rendu principal - appele a chaque message WebSocket
// ============================================================

/**
 * Point d entree du rendu.
 * Appelle toutes les fonctions de mise a jour de l interface.
 */
function rendreDashboard() {
  if (!window.donneesWS) return;

  const snapshot = obtenirSnapshotActuel();

  rendreCartes(snapshot);
  rendreGraphique();
  rendreProcessus(snapshot);
  rendreQuotas();
  rendreSidebar();
  rendreAlertes();
}

// ============================================================
// Selection de la machine a afficher
// ============================================================

/**
 * Retourne le snapshot de la machine actuellement selectionnee.
 * "local" -> snapshot de la machine maitre
 * autre   -> snapshot de l agent selectionne
 *
 * @returns {Object|null} Le snapshot ou null si indisponible
 */
function obtenirSnapshotActuel() {
  if (machineSelectionnee === 'local') {
    return window.donneesWS.local || null;
  }
  const agent = (window.donneesWS.agents || {})[machineSelectionnee];
  return (agent && agent.en_ligne) ? agent.snapshot : null;
}

// ============================================================
// Rendu des 4 cartes de statistiques
// ============================================================

/**
 * Met a jour les 4 cartes en haut du dashboard.
 * @param {Object|null} snapshot - Le snapshot de la machine selectionnee
 */
function rendreCartes(snapshot) {
  if (!snapshot) {
    // Machine hors ligne : afficher des tirets
    document.getElementById('carte-rx').innerHTML = '<small>hors ligne</small>';
    document.getElementById('carte-tx').innerHTML = '<small>hors ligne</small>';
    document.getElementById('carte-procs').textContent = '–';
    return;
  }

  // Formater et afficher le debit entrant
  const rx = formaterOctets(snapshot.total_rx_bps);
  const [rxVal, ...rxUnite] = rx.split(' ');
  document.getElementById('carte-rx').innerHTML =
    rxVal + ' <small>' + rxUnite.join(' ') + '/s</small>';

  // Formater et afficher le debit sortant
  const tx = formaterOctets(snapshot.total_tx_bps);
  const [txVal, ...txUnite] = tx.split(' ');
  document.getElementById('carte-tx').innerHTML =
    txVal + ' <small>' + txUnite.join(' ') + '/s</small>';

  // Nombre de processus surveilles
  document.getElementById('carte-procs').textContent = snapshot.processus.length;

  // Nom de la machine
  document.getElementById('carte-machine').textContent =
    'Machine : ' + (snapshot.machine || '–');
  document.getElementById('label-machine-proc').textContent =
    snapshot.machine || '–';

  // Interface reseau active
  const interfaces = (snapshot.interfaces || [])
    .map(i => i.nom).join(', ') || '–';
  document.getElementById('carte-rx-iface').textContent = interfaces;
  document.getElementById('carte-tx-iface').textContent = interfaces;

  // Nombre d alertes
  document.getElementById('carte-alertes').textContent =
    (window.donneesWS.alertes || []).length;
}

// ============================================================
// Rendu du graphique de debit
// ============================================================

/**
 * Met a jour le graphique avec les donnees d historique du serveur.
 * Delegue a mettreAJourGraphique() definie dans charts.js.
 */
function rendreGraphique() {
  if (!graphiqueDebit) return;
  const rx = window.donneesWS.historique_rx || [];
  const tx = window.donneesWS.historique_tx || [];
  mettreAJourGraphique(graphiqueDebit, rx, tx);
}

// ============================================================
// Rendu du tableau des processus
// ============================================================

/**
 * Remplit le tableau des processus avec les donnees du snapshot.
 * @param {Object|null} snapshot
 */
function rendreProcessus(snapshot) {
  const corps = document.getElementById('corps-tableau-proc');

  if (!snapshot || snapshot.processus.length === 0) {
    corps.innerHTML = `
      <tr>
        <td colspan="7" style="text-align:center;padding:30px;color:var(--texte-leger)">
          Aucune donnée disponible
        </td>
      </tr>`;
    return;
  }

  // Construire une ligne HTML par processus
  corps.innerHTML = snapshot.processus.map(proc => {
    const debitTotal = proc.rx_bps + proc.tx_bps;
    return `
      <tr>
        <td><strong>${echapper(proc.nom)}</strong></td>
        <td style="color:var(--texte-leger);font-size:12px">${proc.pid}</td>
        <td>${proc.cpu_pct.toFixed(1)}%</td>
        <td>${formaterMegaoctets(proc.ram_mb)}</td>
        <td style="color:var(--bleu)">${formaterOctets(proc.rx_bps)}/s</td>
        <td style="color:var(--cyan)">${formaterOctets(proc.tx_bps)}/s</td>
        <td>${badgeNiveau(debitTotal)}</td>
      </tr>`;
  }).join('');
}

// ============================================================
// Rendu des quotas
// ============================================================

/**
 * Affiche les barres de progression des quotas.
 */
function rendreQuotas() {
  const quotas = window.donneesWS.quotas || {};
  const cles   = Object.keys(quotas);
  const conteneur = document.getElementById('panel-quotas');

  if (cles.length === 0) {
    conteneur.innerHTML = `
      <div class="etat-vide">
        <div class="emoji">✅</div>
        <p>Aucun quota configuré<br><small>Ajoutez-en dans Réglages</small></p>
      </div>`;
    return;
  }

  conteneur.innerHTML = cles.map(cle => {
    const q   = quotas[cle];
    // Choisir la couleur selon le niveau d utilisation
    const cls = q.pct >= 100 ? 'depasse' : q.pct >= 80 ? 'avertissement' : '';

    return `
      <div class="quota-item">
        <div class="quota-entete">
          <span class="quota-nom">${echapper(q.machine)}</span>
          <span class="quota-valeur">${q.utilise_mb} / ${q.limite_mb} MB (${q.pct}%)</span>
        </div>
        <div class="barre-fond">
          <div class="barre-rempli ${cls}" style="width:${Math.min(q.pct, 100)}%"></div>
        </div>
        <div class="quota-actions">
          <button class="btn-reset" onclick="resetQuota('${echapper(q.machine)}')">
            Réinitialiser
          </button>
        </div>
      </div>`;
  }).join('');
}

// ============================================================
// Rendu de la sidebar (liste des machines)
// ============================================================

/**
 * Met a jour la liste des agents dans la sidebar.
 * Affiche un point vert si en ligne, gris si hors ligne.
 */
function rendreSidebar() {
  const agents = window.donneesWS.agents || {};

  // Mettre a jour le nom de la machine locale
  if (window.donneesWS.local) {
    document.getElementById('nom-machine-locale').textContent =
      window.donneesWS.local.machine || 'Locale';
  }

  // Generer les elements pour chaque agent
  document.getElementById('liste-agents').innerHTML =
    Object.entries(agents).map(([nom, agent]) => `
      <div class="machine-item ${machineSelectionnee === nom ? 'selectionnee' : ''}"
           data-machine="${echapper(nom)}">
        <div class="point ${agent.en_ligne ? 'ligne' : 'hors'}"></div>
        <span>${echapper(nom)}</span>
      </div>`
    ).join('');
}

// ============================================================
// Rendu des alertes
// ============================================================

/** Icones par type d alerte */
const ICONES_ALERTES = {
  DebitEleve:         '⚡',
  AvertissementQuota: '⚠️',
  QuotaDepasse:       '🚫',
};

/** Libelles par type d alerte */
const LIBELLES_ALERTES = {
  DebitEleve:         'Haut débit',
  AvertissementQuota: 'Quota ⚠',
  QuotaDepasse:       'Quota dépassé',
};

/**
 * Affiche la liste des alertes dans l onglet Alertes.
 * Gere aussi le badge rouge sur le bouton de navigation.
 */
function rendreAlertes() {
  const alertes = window.donneesWS.alertes || [];

  // Compter les nouvelles alertes (non encore vues)
  const nouvelles = alertes.filter(a => {
    const cle = a.horodatage + '|' + a.message;
    return !alertesVues.has(cle);
  });

  // Marquer les nouvelles alertes comme vues
  nouvelles.forEach(a => alertesVues.add(a.horodatage + '|' + a.message));
  nbNouvellesAlertes += nouvelles.length;

  // Afficher le badge si necessaire
  const badge = document.getElementById('badge-alertes');
  if (nbNouvellesAlertes > 0) {
    badge.style.display = 'inline';
    badge.textContent   = nbNouvellesAlertes > 99 ? '99+' : nbNouvellesAlertes;
  }

  // Mettre a jour le compteur dans la carte
  document.getElementById('carte-alertes').textContent = alertes.length;

  // Afficher les alertes (les plus recentes en premier)
  const conteneur = document.getElementById('liste-alertes');
  if (alertes.length === 0) {
    conteneur.innerHTML = `
      <div class="etat-vide">
        <div class="emoji">🎉</div>
        <p>Aucune alerte — tout va bien !</p>
      </div>`;
    return;
  }

  conteneur.innerHTML = [...alertes].reverse().map(a => `
    <div class="alerte-item">
      <div class="alerte-icone">${ICONES_ALERTES[a.type_alerte] || '🔔'}</div>
      <div class="alerte-corps">
        <div class="alerte-msg">${echapper(a.message)}</div>
        <div class="alerte-meta">
          ${echapper(a.machine)} · ${formaterHeure(a.horodatage)}
        </div>
      </div>
      <div class="alerte-type ${echapper(a.type_alerte)}">
        ${echapper(LIBELLES_ALERTES[a.type_alerte] || a.type_alerte)}
      </div>
    </div>`
  ).join('');
}

// ============================================================
// Navigation entre les onglets
// ============================================================

/**
 * Active un onglet et masque les autres.
 * @param {string} nomOnglet - "dashboard", "alertes" ou "reglages"
 */
function activerOnglet(nomOnglet) {
  // Mettre a jour la navigation
  document.querySelectorAll('.nav-item').forEach(el => {
    el.classList.toggle('actif', el.dataset.onglet === nomOnglet);
  });

  // Afficher le bon panneau
  document.querySelectorAll('.panneau').forEach(el => {
    el.classList.toggle('actif', el.id === 'panneau-' + nomOnglet);
  });

  // Mettre a jour le titre
  const titres = {
    dashboard: ['Dashboard',  'Surveillance réseau en temps réel'],
    alertes:   ['Alertes',    'Historique des alertes de la session'],
    reglages:  ['Réglages',   'Agents, seuils et quotas'],
  };
  if (titres[nomOnglet]) {
    document.getElementById('titre-page').textContent      = titres[nomOnglet][0];
    document.getElementById('sous-titre-page').textContent = titres[nomOnglet][1];
  }

  // Charger la config quand on ouvre Reglages
  if (nomOnglet === 'reglages') chargerConfiguration();

  // Remettre le badge a zero quand on ouvre Alertes
  if (nomOnglet === 'alertes') {
    nbNouvellesAlertes = 0;
    document.getElementById('badge-alertes').style.display = 'none';
  }
}

// ============================================================
// Toast de notification dans l interface
// ============================================================

let timerToast = null;

/**
 * Affiche un message temporaire en bas a droite de l ecran.
 * @param {string} message - Le texte a afficher
 */
function afficherToast(message) {
  clearTimeout(timerToast);
  const toast = document.getElementById('toast');
  toast.textContent = message;
  toast.classList.add('visible');
  timerToast = setTimeout(() => toast.classList.remove('visible'), 3200);
}

// Rendre afficherToast disponible pour settings.js
window.afficherToast = afficherToast;

// ============================================================
// Utilitaires de formatage
// ============================================================

/**
 * Formate des megaoctets en chaine lisible.
 * @param {number} mb - Megaoctets
 * @returns {string} Ex: "256 MB" ou "1.25 GB"
 */
function formaterMegaoctets(mb) {
  if (mb < 1024) return mb.toFixed(0) + ' MB';
  return (mb / 1024).toFixed(2) + ' GB';
}

/**
 * Convertit un timestamp Unix en heure lisible.
 * @param {number} ts - Secondes depuis epoch
 * @returns {string} Ex: "14:32:05"
 */
function formaterHeure(ts) {
  if (!ts) return '–';
  return new Date(ts * 1000).toLocaleTimeString('fr-FR');
}

/**
 * Retourne un badge HTML selon le niveau de debit.
 * @param {number} bps - Debit en bytes/seconde
 * @returns {string} HTML du badge
 */
function badgeNiveau(bps) {
  if (bps > 2_000_000) return '<span class="badge eleve">Élevé</span>';
  if (bps > 300_000)   return '<span class="badge moyen">Moyen</span>';
  return '<span class="badge faible">Faible</span>';
}

// Variables CSS pour reutilisation dans les styles JS
document.documentElement.style.setProperty('--bleu', '#4896FE');
document.documentElement.style.setProperty('--cyan', '#16C8C7');

// ============================================================
// Ecouteurs d evenements — configures au chargement de la page
// ============================================================

document.addEventListener('DOMContentLoaded', function() {

  // --- Navigation entre onglets ---
  document.querySelectorAll('.nav-item[data-onglet]').forEach(function(el) {
    el.addEventListener('click', function() {
      activerOnglet(this.dataset.onglet);
    });
  });

  // --- Selection d une machine dans la sidebar ---
  // Delegation d evenement (fonctionne meme si les elements sont ajoutes dynamiquement)
  document.querySelector('.section-machines').addEventListener('click', function(e) {
    const item = e.target.closest('.machine-item');
    if (!item) return;

    machineSelectionnee = item.dataset.machine;

    // Mettre a jour la selection visuelle
    document.querySelectorAll('.machine-item').forEach(el => {
      el.classList.toggle('selectionnee', el === item);
    });

    // Rafraichir l affichage immediatement
    rendreDashboard();
  });

  // --- Bouton effacer toutes les alertes ---
  document.getElementById('btn-effacer-alertes').addEventListener('click', function() {
    alertesVues.clear();
    nbNouvellesAlertes = 0;
    document.getElementById('badge-alertes').style.display = 'none';
    document.getElementById('liste-alertes').innerHTML = `
      <div class="etat-vide">
        <div class="emoji">🎉</div>
        <p>Alertes effacées localement</p>
      </div>`;
  });

  // --- Initialisation ---

  // Creer le graphique (defini dans charts.js)
  graphiqueDebit = creerGraphiqueDebit();

  // Demarrer la connexion WebSocket
  connecterWebSocket();

  console.log('NetWatch dashboard initialise.');
});