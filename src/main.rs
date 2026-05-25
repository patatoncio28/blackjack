use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;
use core_shared::sdk::{GameConfig, GameServer, SdkState, get_balance, mutate_balance};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Card {
    suit: String,
    value: String,
    score: i32,
}

#[derive(Clone, Debug)]
struct BlackjackGame {
    user_id: String,
    deck: Vec<Card>,
    player_hand: Vec<Card>,
    dealer_hand: Vec<Card>,
    bet: i64,
    is_over: bool,
    status_message: String,
}

#[derive(Serialize)]
struct ClientView {
    player_hand: Vec<Card>,
    dealer_hand: Vec<Card>,
    player_score: i32,
    dealer_score: i32,
    bet: i64,
    is_over: bool,
    status_message: String,
    new_balance: i64,
}

#[derive(Deserialize)]
struct PlayRequest {
    user_id: String,
    bet: i64,
}

#[derive(Deserialize)]
struct ActionRequest {
    user_id: String,
}

static ACTIVE_GAMES: OnceLock<Mutex<HashMap<String, BlackjackGame>>> = OnceLock::new();

fn get_active_games() -> &'static Mutex<HashMap<String, BlackjackGame>> {
    ACTIVE_GAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn create_deck() -> Vec<Card> {
    let suits = vec!["♥", "♦", "♣", "♠"];
    let values = vec![
        ("2", 2), ("3", 3), ("4", 4), ("5", 5), ("6", 6), ("7", 7), ("8", 8), ("9", 9), ("10", 10),
        ("J", 10), ("Q", 10), ("K", 10), ("A", 11)
    ];
    let mut deck = Vec::new();
    for suit in &suits {
        for (val, score) in &values {
            deck.push(Card {
                suit: suit.to_string(),
                value: val.to_string(),
                score: *score,
            });
        }
    }
    deck
}

fn shuffle_deck(deck: &mut Vec<Card>) {
    let mut rng = rand::thread_rng();
    deck.shuffle(&mut rng);
}

fn calculate_score(hand: &[Card]) -> i32 {
    let mut score = 0;
    let mut aces = 0;
    for card in hand {
        score += card.score;
        if card.value == "A" {
            aces += 1;
        }
    }
    while score > 21 && aces > 0 {
        score -= 10;
        aces -= 1;
    }
    score
}

fn make_client_view(game: &BlackjackGame, balance: i64) -> ClientView {
    let mut dealer_visible = game.dealer_hand.clone();
    if !game.is_over && dealer_visible.len() > 1 {
        dealer_visible.truncate(1);
    }
    let player_score = calculate_score(&game.player_hand);
    let dealer_score = calculate_score(&dealer_visible);

    ClientView {
        player_hand: game.player_hand.clone(),
        dealer_hand: dealer_visible,
        player_score,
        dealer_score,
        bet: game.bet,
        is_over: game.is_over,
        status_message: game.status_message.clone(),
        new_balance: balance,
    }
}

async fn stand_round_internal(state: &Arc<SdkState>, user_id: &str) -> Result<ClientView, String> {
    let mut game = {
        let active = get_active_games().lock().unwrap();
        match active.get(user_id) {
            Some(g) => g.clone(),
            None => return Err("No hay ninguna partida activa".to_string()),
        }
    };

    if game.is_over {
        return Err("La partida ya ha finalizado".to_string());
    }

    game.is_over = true;
    let player_score = calculate_score(&game.player_hand);
    let mut dealer_score = calculate_score(&game.dealer_hand);

    while dealer_score < 17 && !game.deck.is_empty() {
        game.dealer_hand.push(game.deck.pop().unwrap());
        dealer_score = calculate_score(&game.dealer_hand);
    }

    let current_balance = get_balance(state, user_id).await;
    let mut payout = 0;

    if dealer_score > 21 {
        payout = game.bet * 2;
        game.status_message = format!("¡El crupier se pasa con {}! Ganaste ${}.", dealer_score, game.bet);
    } else if player_score > dealer_score {
        payout = game.bet * 2;
        game.status_message = format!("¡Ganaste! Tu {} supera al {} del crupier.", player_score, dealer_score);
    } else if player_score < dealer_score {
        payout = 0;
        game.status_message = format!("Perdiste. El {} del crupier supera tu {}.", dealer_score, player_score);
    } else {
        payout = game.bet;
        game.status_message = format!("Empate con {}. Se devuelve tu apuesta.", player_score);
    }

    let final_balance = current_balance + payout;
    if payout > 0 {
        mutate_balance(state, user_id, final_balance).await;
    }

    {
        let mut active = get_active_games().lock().unwrap();
        active.insert(user_id.to_string(), game.clone());
    }

    Ok(make_client_view(&game, final_balance))
}

async fn play(
    State(state): State<Arc<SdkState>>,
    Json(payload): Json<PlayRequest>,
) -> impl IntoResponse {
    let user_id = &payload.user_id;
    let bet = payload.bet;

    if bet <= 0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "La apuesta debe ser mayor que 0" })),
        )
            .into_response();
    }

    let current_balance = get_balance(&state, user_id).await;
    if current_balance < bet {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Saldo insuficiente para esta apuesta" })),
        )
            .into_response();
    }

    let deducted_balance = current_balance - bet;
    mutate_balance(&state, user_id, deducted_balance).await;

    let mut deck = create_deck();
    shuffle_deck(&mut deck);

    let mut player_hand = Vec::new();
    let mut dealer_hand = Vec::new();

    // Deal alternating
    if deck.len() >= 4 {
        player_hand.push(deck.pop().unwrap());
        dealer_hand.push(deck.pop().unwrap());
        player_hand.push(deck.pop().unwrap());
        dealer_hand.push(deck.pop().unwrap());
    }

    let player_score = calculate_score(&player_hand);
    let dealer_score = calculate_score(&dealer_hand);

    let mut is_over = false;
    let mut status_message = "Tu turno: Pide carta (Hit) o Plántate (Stand)".to_string();
    let mut final_balance = deducted_balance;

    if player_score == 21 {
        is_over = true;
        if dealer_score == 21 {
            status_message = "¡Empate! Ambos tienen Blackjack.".to_string();
            final_balance += bet;
        } else {
            let payout = (bet as f64 * 2.5) as i64;
            status_message = format!("¡Blackjack Natural! Ganaste ${}", payout - bet);
            final_balance += payout;
        }
        mutate_balance(&state, user_id, final_balance).await;
    }

    let game = BlackjackGame {
        user_id: user_id.clone(),
        deck,
        player_hand,
        dealer_hand,
        bet,
        is_over,
        status_message,
    };

    {
        let mut active = get_active_games().lock().unwrap();
        active.insert(user_id.clone(), game.clone());
    }

    Json(make_client_view(&game, final_balance)).into_response()
}

async fn hit(
    State(state): State<Arc<SdkState>>,
    Json(payload): Json<ActionRequest>,
) -> impl IntoResponse {
    let user_id = &payload.user_id;

    let mut game = {
        let active = get_active_games().lock().unwrap();
        match active.get(user_id) {
            Some(g) => g.clone(),
            None => {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "No hay ninguna partida activa" })),
                )
                    .into_response()
            }
        }
    };

    if game.is_over {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "La partida ya ha finalizado" })),
        )
            .into_response();
    }

    if game.deck.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "El mazo está vacío" })),
        )
            .into_response();
    }

    game.player_hand.push(game.deck.pop().unwrap());
    let score = calculate_score(&game.player_hand);
    let current_balance = get_balance(&state, user_id).await;

    if score > 21 {
        game.is_over = true;
        game.status_message = format!("¡Bust con {}! Has superado 21 y has perdido.", score);
        {
            let mut active = get_active_games().lock().unwrap();
            active.insert(user_id.clone(), game.clone());
        }
        Json(make_client_view(&game, current_balance)).into_response()
    } else if score == 21 {
        // Auto-stand
        match stand_round_internal(&state, user_id).await {
            Ok(view) => Json(view).into_response(),
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response(),
        }
    } else {
        game.status_message = "Has pedido carta. ¿Hit o Stand?".to_string();
        {
            let mut active = get_active_games().lock().unwrap();
            active.insert(user_id.clone(), game.clone());
        }
        Json(make_client_view(&game, current_balance)).into_response()
    }
}

async fn stand(
    State(state): State<Arc<SdkState>>,
    Json(payload): Json<ActionRequest>,
) -> impl IntoResponse {
    let user_id = &payload.user_id;
    match stand_round_internal(&state, user_id).await {
        Ok(view) => Json(view).into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn get_player_balance(
    State(state): State<Arc<SdkState>>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let balance = get_balance(&state, &user_id).await;
    Json(serde_json::json!({ "user_id": user_id, "balance": balance }))
}

#[tokio::main]
async fn main() {
    let config = GameConfig {
        id: "blackjack",
        name: "Blackjack Pro",
        description: "Supera al crupier en este juego clásico de cartas.",
        category: "Cartas",
    };

    let server = GameServer::new(config)
        .route("/api/games/blackjack/play", post(play))
        .route("/api/games/blackjack/hit", post(hit))
        .route("/api/games/blackjack/stand", post(stand))
        .route("/api/games/blackjack/balance/:user_id", get(get_player_balance))
        .static_dir("/api/games/blackjack/ui", "ui");

    server.run().await;
}
