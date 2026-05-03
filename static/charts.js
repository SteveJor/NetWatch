// Nombre maximum de points affichés sur le graphique.
const MAX_POINTS = 20;

// Référence au graphique Chart.js de débit.
let graphiqueDebit = null;

// Initialise le graphique réseau et retourne l'instance Chart.
function creerGraphiqueDebit() {
  const contexte = document.getElementById('graphique-debit')?.getContext('2d');
  if (!contexte) return null;

  graphiqueDebit = new Chart(contexte, {
    type: 'line',
    data: {
      labels: Array(MAX_POINTS).fill(''),
      datasets: [
        {
          label: 'Rx (Mo/s)',
          data: Array(MAX_POINTS).fill(0),
          backgroundColor: 'rgba(56, 189, 248, 0.2)',
          borderColor: '#38bdf8',
          borderWidth: 2,
          fill: true,
          tension: 0.25,
        },
        {
          label: 'Tx (Mo/s)',
          data: Array(MAX_POINTS).fill(0),
          backgroundColor: 'rgba(16, 185, 129, 0.2)',
          borderColor: '#10b981',
          borderWidth: 2,
          fill: true,
          tension: 0.25,
        }
      ]
    },
    options: {
      animation: false,
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: { display: false },
        y: { beginAtZero: true }
      },
      plugins: {
        tooltip: {
          callbacks: {
            label: context => `${context.dataset.label}: ${context.formattedValue} Mo/s`
          }
        }
      }
    }
  });

  return graphiqueDebit;
}

// Décale les données vers la gauche et ajoute la nouvelle valeur.
function deplacerDonnees(dataset, valeur) {
  dataset.data.push(valeur);
  if (dataset.data.length > MAX_POINTS) {
    dataset.data.shift();
  }
}

// Met à jour le graphique avec les nouvelles données reçues.
function mettreAJourGraphique(donnees = {}) {
  if (graphiqueDebit && donnees.rx != null && donnees.tx != null) {
    const [rxDataset, txDataset] = graphiqueDebit.data.datasets;
    deplacerDonnees(rxDataset, donnees.rx);
    deplacerDonnees(txDataset, donnees.tx);
    graphiqueDebit.update('none');
  }
}

export { creerGraphiqueDebit, mettreAJourGraphique };
