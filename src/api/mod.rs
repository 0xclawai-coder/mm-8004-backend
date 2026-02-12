use axum::Router;

use crate::AppState;

pub mod activity;
pub mod agents;
pub mod leaderboard;
pub mod marketplace;
pub mod stats;

/// Build the /api router with all sub-routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(activity::router())
        .merge(agents::router())
        .merge(leaderboard::router())
        .merge(marketplace::router())
        .merge(stats::router())
}
