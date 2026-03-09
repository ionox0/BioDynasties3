//! AI goal system — a two-phase generate/execute pipeline.
//!
//! ## Design pattern
//!
//! Each frame the AI runs in two distinct phases:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  GENERATE  (strategy systems — read-only)               │
//! │                                                         │
//! │  worker_goal_system     ──► GlobalGoalManager           │
//! │  production_goal_system ──► GlobalGoalManager           │
//! │  (future: combat, expansion, …)                         │
//! └────────────────────────────┬────────────────────────────┘
//!                              │  Vec<PrioritizedGoal>
//! ┌────────────────────────────▼────────────────────────────┐
//! │  EXECUTE   (executor — events only, no component writes)│
//! │                                                         │
//! │  execute_ai_goals_system drains sorted goals            │
//! │    AssignWorkerToResource → SetTargetResourceEvent      │
//! │                           + MovementTargetEvent         │
//! │    BuildUnit              → QueueProductionEvent        │
//! │                             (deducts cost; event       │
//! │                              consumed by ProductionPlugin)│
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Why two phases?
//!
//! Strategy systems only **read** game state; they never fire events or mutate
//! components. This makes each strategy module easy to reason about in isolation:
//! the only output is a set of `PrioritizedGoal` values pushed onto
//! `GlobalGoalManager`. The executor then translates those goals into the
//! existing event interface used by player input — the gathering loop, movement
//! system, and production queue are all player/AI agnostic.
//!
//! Separating strategy from execution also makes it trivial to add new
//! behaviors: write a strategy system that pushes goals, register it in
//! `AIPlugin`; the executor handles delivery automatically.
//!
//! ### Priority
//!
//! Goals carry a `f32` priority score. `GlobalGoalManager::drain_sorted`
//! returns them highest-first, so when multiple strategies generate goals in
//! the same frame the executor processes the most urgent ones first. This
//! gives a cheap way to express "worker assignment is more important than
//! building production" without a full planner.
//!
//! ### Per-frame lifetime
//!
//! `GlobalGoalManager` is drained completely every frame — stale goals never
//! accumulate. Strategy systems regenerate goals fresh each tick based on
//! current game state. This keeps the AI reactive: a worker that becomes idle
//! mid-frame is reassigned within one tick without any explicit cancellation
//! logic.
//!
//! ### Adding a new goal type
//!
//! 1. Add a variant to `UnifiedGoal` in `goals/types.rs`.
//! 2. Write a strategy system in `strategy/` that reads game state and calls
//!    `goals.push(priority, UnifiedGoal::YourVariant { … })`.
//! 3. Add a match arm in `execute_ai_goals_system` that fires the appropriate
//!    existing events (never mutate components directly).
//! 4. Register the strategy system in `AIPlugin::build`.

pub mod types;
mod executors;

pub use executors::execute_ai_goals_system;
