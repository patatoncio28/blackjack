import { gameSDK } from '/js/game-sdk.js';

// Initialize dynamic dark neon header
gameSDK.injectHeader("Blackjack Pro");

// Game state variables
let selectedBet = 10;
let gameActive = false;

// DOM elements cache
const el = {
    playerCards: document.getElementById('player-cards'),
    dealerCards: document.getElementById('dealer-cards'),
    playerScore: document.getElementById('player-score'),
    dealerScore: document.getElementById('dealer-score'),
    roundStatus: document.getElementById('round-status'),
    btnDeal: document.getElementById('btn-deal'),
    btnHit: document.getElementById('btn-hit'),
    btnStand: document.getElementById('btn-stand'),
    chipsSelector: document.getElementById('chips-selector'),
    currentBetDisplay: document.getElementById('current-bet-display')
};

// 1. Chip interaction bindings
el.chipsSelector.addEventListener('click', (e) => {
    if (gameActive) return; // Cannot modify bet during active round
    
    const targetChip = e.target.closest('.chip');
    if (!targetChip) return;

    // Remove active from others
    el.chipsSelector.querySelectorAll('.chip').forEach(c => c.classList.remove('active'));
    targetChip.classList.add('active');

    selectedBet = parseInt(targetChip.dataset.amount, 10);
    el.currentBetDisplay.textContent = `$${selectedBet}`;
});

// Helper to format/render single card element
function createCardElement(card, isHidden = false) {
    const div = document.createElement('div');
    if (isHidden) {
        div.className = 'card-visual face-down';
        return div;
    }

    const isRed = card.suit === '♥' || card.suit === '♦';
    div.className = `card-visual ${isRed ? 'red-card' : 'dark-card'}`;
    
    div.innerHTML = `
        <div class="card-top">
            <span>${card.value}</span>
            <span>${card.suit}</span>
        </div>
        <div class="card-suit-center">${card.suit}</div>
        <div class="card-bottom">
            <span>${card.value}</span>
            <span>${card.suit}</span>
        </div>
    `;
    return div;
}

// Render hands and scores
function updateUI(state) {
    // Render Player Cards
    el.playerCards.innerHTML = '';
    state.player_hand.forEach(c => {
        el.playerCards.appendChild(createCardElement(c));
    });
    el.playerScore.textContent = state.player_score;

    // Render Dealer Cards
    el.dealerCards.innerHTML = '';
    state.dealer_hand.forEach(c => {
        el.dealerCards.appendChild(createCardElement(c));
    });
    // If active and only one card showing, add a visual card facing down
    if (!state.is_over && state.dealer_hand.length === 1) {
        el.dealerCards.appendChild(createCardElement(null, true));
        el.dealerScore.textContent = `${state.dealer_score} + ?`;
    } else {
        el.dealerScore.textContent = state.dealer_score;
    }

    // Status message
    el.roundStatus.textContent = state.status_message;
    el.roundStatus.className = 'status-message';

    if (state.is_over) {
        gameActive = false;
        el.btnDeal.disabled = false;
        el.btnHit.disabled = true;
        el.btnStand.disabled = true;
        el.chipsSelector.style.opacity = '1';
        
        // Color status depending on win/loss
        if (state.status_message.includes('Ganaste') || state.status_message.includes('Blackjack Natural')) {
            el.roundStatus.classList.add('won');
        } else if (state.status_message.includes('Perdiste') || state.status_message.includes('Bust')) {
            el.roundStatus.classList.add('lost');
        }

        // Sincronizar balance contra el Hub
        gameSDK.fetchBalance();
    } else {
        gameActive = true;
        el.btnDeal.disabled = true;
        el.btnHit.disabled = false;
        el.btnStand.disabled = false;
        el.chipsSelector.style.opacity = '0.4';
    }
}

// Start new round
el.btnDeal.addEventListener('click', async () => {
    if (gameActive) return;

    // Optimistically disable double-clicking
    el.btnDeal.disabled = true;
    el.roundStatus.textContent = "Repartiendo cartas...";

    try {
        const response = await gameSDK.request('play', 'POST', {
            user_id: gameSDK.userId,
            bet: selectedBet
        });
        
        updateUI(response);
    } catch (e) {
        el.roundStatus.textContent = `Error: ${e.message}`;
        el.btnDeal.disabled = false;
    }
});

// Hit card action
el.btnHit.addEventListener('click', async () => {
    if (!gameActive) return;

    try {
        const response = await gameSDK.request('hit', 'POST', {
            user_id: gameSDK.userId
        });
        
        updateUI(response);
    } catch (e) {
        el.roundStatus.textContent = `Error: ${e.message}`;
    }
});

// Stand action
el.btnStand.addEventListener('click', async () => {
    if (!gameActive) return;

    try {
        const response = await gameSDK.request('stand', 'POST', {
            user_id: gameSDK.userId
        });
        
        updateUI(response);
    } catch (e) {
        el.roundStatus.textContent = `Error: ${e.message}`;
    }
});
