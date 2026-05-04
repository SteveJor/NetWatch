// ============================================================
// Membre 9 - Formulaires de configuration du dashboard
//
// Ce fichier gere tout l onglet "Reglages" :
//   - Ajout / suppression d agents
//   - Ajout / suppression de quotas
//   - Reglage des seuils d alerte
//   - Sauvegarde de la configuration via l API
//
// Il utilise la variable globale `configActuelle` definie dans app.js.
// ============================================================

// ============================================================
// Chargement de la configuration depuis le serveur
// ============================================================

/**
 * Charge la configuration actuelle depuis /api/config
 * et remplit les formulaires de l onglet Reglages.
 */
async function chargerConfiguration() {
  try {
    const reponse = await fetch('/api/config');
    if (!reponse.ok) throw new Error('Erreur HTTP ' + reponse.status);

    // Stocker dans la variable globale (definie dans app.js)
    window.configActuelle = await reponse.json();

    // Remplir les formulaires avec les valeurs recues
    afficherAgents();
    afficherQuotas();
    afficherSeuils();

  } catch (erreur) {
    afficherToast('❌ Impossible de charger la configuration : ' + erreur.message);
  }
}

// ============================================================
// Affichage des agents dans le tableau des reglages
// ============================================================

/**
 * Met a jour le tableau HTML des agents.
 * Utilise window.donneesWS (donnees WebSocket) pour le statut en ligne.
 */
function afficherAgents() {
  const config  = window.configActuelle;
  const donnees = window.donneesWS;
  const corps   = document.getElementById('corps-agents');

  if (!config || config.agents.length === 0) {
    corps.innerHTML = `
      <tr>
        <td colspan="4" style="text-align:center;padding:16px;color:var(--texte-leger)">
          Aucun agent configuré
        </td>
      </tr>`;
    return;
  }

  corps.innerHTML = config.agents.map((agent, index) => {
    // Verifier si l agent est en ligne (depuis les donnees WebSocket)
    const statut     = donnees && donnees.agents && donnees.agents[agent.nom];
    const enLigne    = statut && statut.en_ligne;
    const badgeStyle = enLigne
      ? 'color:var(--succes);font-weight:600'
      : 'color:var(--texte-leger)';
    const texteStatut = enLigne ? '● En ligne' : '○ Hors ligne';

    return `
      <tr>
        <td>${echapper(agent.nom)}</td>
        <td><code>${echapper(agent.ip)}</code></td>
        <td><span style="${badgeStyle}">${texteStatut}</span></td>
        <td>
          <button class="btn btn-danger" onclick="supprimerAgent(${index})">
            Retirer
          </button>
        </td>
      </tr>`;
  }).join('');
}

// ============================================================
// Affichage des quotas dans le tableau des reglages
// ============================================================

/**
 * Met a jour le tableau HTML des quotas configurés.
 * Affiche aussi le quota actuellement consomme si disponible.
 */
function afficherQuotas() {
  const config  = window.configActuelle;
  const donnees = window.donneesWS;
  const corps   = document.getElementById('corps-quotas');

  if (!config || config.quotas.length === 0) {
    corps.innerHTML = `
      <tr>
        <td colspan="4" style="text-align:center;padding:16px;color:var(--texte-leger)">
          Aucun quota configuré
        </td>
      </tr>`;
    return;
  }

  corps.innerHTML = config.quotas.map((quota, index) => {
    // Recuperer l usage actuel depuis les donnees temps reel
    const etat     = donnees && donnees.quotas && donnees.quotas[quota.machine];
    const utilise  = etat ? etat.utilise_mb + ' MB' : '–';

    return `
      <tr>
        <td>${echapper(quota.machine)}</td>
        <td>${quota.limite_mb} MB</td>
        <td>${utilise}</td>
        <td>
          <button class="btn btn-danger" onclick="supprimerQuota(${index})">
            Retirer
          </button>
        </td>
      </tr>`;
  }).join('');
}

// ============================================================
// Affichage des seuils d alerte
// ============================================================

/**
 * Remplit les sliders avec les valeurs de la configuration.
 */
function afficherSeuils() {
  const config = window.configActuelle;
  if (!config) return;

  const sliderDebit    = document.getElementById('slider-debit');
  const sliderQuotaPct = document.getElementById('slider-quota-pct');

  sliderDebit.value    = config.alertes.debit_max_mbps;
  sliderQuotaPct.value = config.alertes.avertissement_quota_pct;

  document.getElementById('val-debit').textContent =
    parseFloat(config.alertes.debit_max_mbps).toFixed(1);
  document.getElementById('val-quota-pct').textContent =
    config.alertes.avertissement_quota_pct;
}

// ============================================================
// Actions sur les agents
// ============================================================

/**
 * Valide et ajoute un agent a la configuration locale.
 * La sauvegarde se fait via le bouton "Sauvegarder".
 */
function ajouterAgent() {
  const nom = document.getElementById('champ-agent-nom').value.trim();
  const ip  = document.getElementById('champ-agent-ip').value.trim();

  // Validation : champs obligatoires
  if (!nom || !ip) {
    afficherToast('⚠️ Le nom et l\'IP sont obligatoires');
    return;
  }

  // Validation : format de l adresse IP
  if (!estIPValide(ip)) {
    afficherToast('❌ Adresse IP invalide. Exemple valide : 192.168.1.101');
    return;
  }

  // Ajouter a la config locale
  if (!window.configActuelle) return;
  window.configActuelle.agents.push({ nom, ip });

  // Vider les champs
  document.getElementById('champ-agent-nom').value = '';
  document.getElementById('champ-agent-ip').value  = '';

  // Rafraichir le tableau
  afficherAgents();
  afficherToast('Agent ajouté — pensez à sauvegarder');
}

/**
 * Supprime un agent de la configuration locale par son index.
 * @param {number} index - Position dans le tableau agents
 */
function supprimerAgent(index) {
  if (!window.configActuelle) return;
  window.configActuelle.agents.splice(index, 1);
  afficherAgents();
  afficherToast('Agent retiré — pensez à sauvegarder');
}

// ============================================================
// Actions sur les quotas
// ============================================================

/**
 * Valide et ajoute un quota a la configuration locale.
 */
function ajouterQuota() {
  const machine  = document.getElementById('champ-quota-machine').value.trim();
  const limiteMb = parseInt(document.getElementById('champ-quota-limite').value);

  if (!machine || !limiteMb || limiteMb < 1) {
    afficherToast('⚠️ La machine et la limite (en MB) sont obligatoires');
    return;
  }

  if (!window.configActuelle) return;
  window.configActuelle.quotas.push({ machine, limite_mb: limiteMb });

  // Vider les champs
  document.getElementById('champ-quota-machine').value = '';
  document.getElementById('champ-quota-limite').value  = '';

  afficherQuotas();
  afficherToast('Quota ajouté — pensez à sauvegarder');
}

/**
 * Supprime un quota de la configuration locale par son index.
 * @param {number} index - Position dans le tableau quotas
 */
function supprimerQuota(index) {
  if (!window.configActuelle) return;
  window.configActuelle.quotas.splice(index, 1);
  afficherQuotas();
  afficherToast('Quota retiré — pensez à sauvegarder');
}

// ============================================================
// Sauvegarde de la configuration
// ============================================================

/**
 * Envoie la configuration modifiee au serveur via l API POST /api/config.
 * Le serveur sauvegarde dans netwatch.toml et l applique immediatement.
 */
async function sauvegarderConfiguration() {
  if (!window.configActuelle) {
    afficherToast('❌ Aucune configuration a sauvegarder');
    return;
  }

  // Lire les valeurs des sliders
  window.configActuelle.alertes.debit_max_mbps =
    parseFloat(document.getElementById('slider-debit').value);
  window.configActuelle.alertes.avertissement_quota_pct =
    parseInt(document.getElementById('slider-quota-pct').value);

  try {
    const reponse = await fetch('/api/config', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify(window.configActuelle),
    });

    const resultat = await reponse.json();

    if (resultat.ok) {
      afficherToast('✅ ' + resultat.message);
    } else {
      afficherToast('❌ ' + resultat.message);
    }

  } catch (erreur) {
    afficherToast('❌ Erreur reseau : ' + erreur.message);
  }
}

/**
 * Remet a zero le quota d une machine via l API.
 * @param {string} machine - Hostname de la machine
 */
async function resetQuota(machine) {
  try {
    const reponse = await fetch('/api/quotas/reset', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ machine }),
    });
    const resultat = await reponse.json();
    if (resultat.ok) {
      afficherToast('✅ Quota réinitialisé pour ' + machine);
    }
  } catch (erreur) {
    afficherToast('❌ Erreur : ' + erreur.message);
  }
}

// ============================================================
// Validation
// ============================================================

/**
 * Verifie si une chaine est une adresse IPv4 valide.
 * Exemple valide : 192.168.1.101
 * Exemple invalide : 999.0.0.1 ou "abc"
 *
 * @param {string} ip - La chaine a valider
 * @returns {boolean} true si l IP est valide
 */
function estIPValide(ip) {
  // Format : 4 groupes de 1 a 3 chiffres separes par des points
  const pattern = /^(\d{1,3}\.){3}\d{1,3}$/;
  if (!pattern.test(ip)) return false;

  // Chaque groupe doit etre entre 0 et 255
  return ip.split('.').every(partie => {
    const nombre = parseInt(partie, 10);
    return nombre >= 0 && nombre <= 255;
  });
}

// ============================================================
// Ecouteurs d evenements - configures au chargement de la page
// ============================================================

document.addEventListener('DOMContentLoaded', function() {

  // Bouton ajout agent
  document.getElementById('btn-ajouter-agent')
    .addEventListener('click', ajouterAgent);

  // Bouton ajout quota
  document.getElementById('btn-ajouter-quota')
    .addEventListener('click', ajouterQuota);

  // Bouton sauvegarder
  document.getElementById('btn-sauvegarder')
    .addEventListener('click', sauvegarderConfiguration);

  // Mise a jour de l affichage du slider debit
  document.getElementById('slider-debit').addEventListener('input', function() {
    document.getElementById('val-debit').textContent =
      parseFloat(this.value).toFixed(1);
  });

  // Mise a jour de l affichage du slider quota %
  document.getElementById('slider-quota-pct').addEventListener('input', function() {
    document.getElementById('val-quota-pct').textContent = this.value;
  });

});

// ============================================================
// Utilitaire
// ============================================================

/**
 * Echappe les caracteres HTML dangereux pour eviter les injections XSS.
 * @param {string} texte - Le texte brut
 * @returns {string} Le texte securise
 */
function echapper(texte) {
  return String(texte ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// Rendre echapper() disponible pour app.js
window.echapper = echapper;
