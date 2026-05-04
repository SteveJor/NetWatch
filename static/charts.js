
/**
 * Formate un nombre d octets en chaine lisible.
 * 0        -> "0 B"
 * 1500     -> "1.5 KB"
 * 2500000  -> "2.50 MB"
 */
function formaterOctets(octets) {
    if (!octets || isNaN(octets) || octets < 0) return '0 B';
    if (octets < 1024)       return octets.toFixed(0) + ' B';
    if (octets < 1048576)    return (octets / 1024).toFixed(1) + ' KB';
    if (octets < 1073741824) return (octets / 1048576).toFixed(2) + ' MB';
    return (octets / 1073741824).toFixed(2) + ' GB';
}

// Rendre disponible globalement pour app.js
window.formaterOctets = formaterOctets;

/**
 * Cree le graphique Chart.js du debit reseau.
 * Appelee une seule fois au demarrage.
 */
function creerGraphiqueDebit() {
    const canvas   = document.getElementById('graphique-debit');
    if (!canvas) return null;
    const contexte = canvas.getContext('2d');

    return new Chart(contexte, {
        type: 'line',
        data: {
            labels: Array(60).fill(''),
            datasets: [
                {
                    label: '↓ Réception',
                    data:  Array(60).fill(0),
                    borderColor:     '#4896FE',
                    backgroundColor: 'rgba(72, 150, 254, 0.1)',
                    fill:       true,
                    tension:    0.4,
                    borderWidth: 2,
                    pointRadius: 0,
                    pointHoverRadius: 4,
                },
                {
                    label: '↑ Envoi',
                    data:  Array(60).fill(0),
                    borderColor:     '#16C8C7',
                    backgroundColor: 'rgba(22, 200, 199, 0.1)',
                    fill:       true,
                    tension:    0.4,
                    borderWidth: 2,
                    pointRadius: 0,
                    pointHoverRadius: 4,
                }
            ]
        },
        options: {
            animation:             false,
            responsive:            true,
            maintainAspectRatio:   false,
            interaction: { intersect: false, mode: 'index' },
            scales: {
                x: { display: false },
                y: {
                    beginAtZero: true,
                    // ── FIX : adapter l echelle automatiquement ──────
                    // Chart.js choisit l echelle selon les vraies valeurs
                    // grace a suggestedMax = 0 (pas de min artificiel)
                    suggestedMin: 0,
                    grid:  { color: 'rgba(0,0,0,0.04)' },
                    ticks: {
                        callback:     v => formaterOctets(v) + '/s',
                        font:         { size: 11 },
                        color:        '#6b7a99',
                        maxTicksLimit: 5,
                    }
                }
            },
            plugins: {
                legend: {
                    position: 'top',
                    labels: {
                        font: { size: 12 }, color: '#1a2332',
                        padding: 16, boxWidth: 12, usePointStyle: true,
                    }
                },
                tooltip: {
                    callbacks: {
                        label: c => c.dataset.label + ' : ' + formaterOctets(c.raw) + '/s'
                    }
                }
            }
        }
    });
}

/**
 * Met a jour le graphique avec l historique des 60 dernieres secondes.
 *
 * @param {Chart}    graphique - Instance Chart.js
 * @param {number[]} histRx    - Tableau du debit entrant (bytes/s)
 * @param {number[]} histTx    - Tableau du debit sortant (bytes/s)
 */
function mettreAJourGraphique(graphique, histRx, histTx) {
    if (!graphique) return;

    // Completer avec des zeros si moins de 60 points
    const rx = completerTableau(histRx, 60);
    const tx = completerTableau(histTx, 60);

    graphique.data.datasets[0].data = rx;
    graphique.data.datasets[1].data = tx;

    // 'none' = pas d animation pour la fluidite
    graphique.update('none');
}

/**
 * Complete un tableau avec des zeros en debut pour atteindre `taille`.
 * [1, 2, 3] avec taille=5 -> [0, 0, 1, 2, 3]
 */
function completerTableau(tableau, taille) {
    const manquants = Math.max(0, taille - tableau.length);
    return Array(manquants).fill(0).concat(tableau).slice(-taille);
}