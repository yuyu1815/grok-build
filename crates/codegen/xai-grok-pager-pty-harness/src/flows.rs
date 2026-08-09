//! Cross-suite e2e flow helpers over [`PtyHarness`] / [`ContentController`].
//!
//! The single canonical home for driving/seeding helpers shared by the
//! pager's `pty_e2e` and `leader_pty_e2e` test targets (both depend on this
//! crate); suite-local constants (sizes, sentinels, timeouts) stay in each
//! suite's `common.rs`.

use std::time::{Duration, Instant};

use crate::{ContentController, MockModel, PtyHarness};

/// Pump PTY output until every label in `labels` is absent from the screen, or
/// until `timeout` elapses. Reattach tests use this to wait out a slow replay
/// before the negative spinner asserts — a fixed-duration settle would flake
/// under host load. On timeout it returns and lets the caller's assert produce
/// the rich screen-dump failure.
pub fn wait_for_labels_absent(h: &mut PtyHarness, labels: &[&str], timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if labels.iter().all(|l| !h.contains_text(l)) {
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        h.update(Duration::from_millis(100));
    }
}

/// Submit `prompt` from `h`, then keep re-pressing Enter until the turn
/// actually starts streaming (`sentinel` appears) or `timeout` elapses.
///
/// In a heavy multi-client leader cluster the driver's submit Enter can be
/// dropped when it races the other client attaching / replaying on the shared
/// leader: the typed prompt is left sitting unsubmitted in the composer, the
/// turn never starts, and a plain `wait_for_text` then times out (the observed
/// `leader_two_clients_shared_session` flake — A idle with `again` still in the
/// composer at 75s). Re-pressing Enter is safe and idempotent: submitting takes
/// the composer draft synchronously (`std::mem::take` in `dispatch`), so once a
/// turn has really been sent the composer is empty and an extra Enter is a
/// no-op. It can only submit a still-stuck prompt, never double-submit a sent
/// one (which would break exactly-once scrollback asserts).
pub fn submit_turn(h: &mut PtyHarness, prompt: &str, sentinel: &str, timeout: Duration) {
    h.inject_keys(format!("{prompt}\r").as_bytes())
        .expect("inject prompt submit");
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        // Per-attempt sub-budget, generous enough that a genuinely in-flight
        // submit resolves before we re-nudge (so the re-nudge only ever fires
        // on an empty composer, where it is a no-op).
        if h.wait_for_text(sentinel, Duration::from_secs(10).min(remaining))
            .is_ok()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out after {timeout:?} waiting for {sentinel:?}\nscreen:\n{}",
            h.screen_contents()
        );
        let _ = h.inject_keys(b"\r");
    }
}

/// Count only inference requests (chat completions / responses / messages),
/// ignoring incidental GETs like /v1/models and /v1/settings, so a replay
/// invariant means "no turn was re-driven" rather than "no HTTP at all".
pub fn inference_request_count(content: &ContentController) -> usize {
    content
        .requests()
        .iter()
        .filter(|e| {
            e.path.contains("/chat/completions")
                || e.path.contains("/responses")
                || e.path.contains("/messages")
        })
        .count()
}

/// Seed a fake xAI OAuth entry into the isolated home's `auth.json` so the
/// shell has session auth (the harness's `XAI_API_KEY` is ApiKey/BYOK mode
/// and never enters the auth manager). Load-bearing details: the scope key
/// must be `<issuer>::<client_id>`, `auth_mode` must be `oidc`, and
/// `expires_at` must be far-future so no network refresh is attempted; the
/// mock server accepts any bearer. Pair with [`oauth_env_for_pager`].
pub fn seed_fake_oauth(content: &ContentController, user: &str) {
    let grok_home = content.home().join(".grok");
    let auth_dir = grok_home.join("auth");
    std::fs::create_dir_all(&auth_dir).expect("create temp .grok/auth");
    std::fs::write(
        auth_dir.join("grok.json"),
        format!(
            r#"{{
  "https://auth.x.ai::b1a00492-073a-47ea-816f-4c329264a828": {{
    "key": "pty-test-oauth-token",
    "auth_mode": "oidc",
    "create_time": "2026-01-01T00:00:00Z",
    "user_id": "{user}",
    "email": "{user}@test.invalid",
    "expires_at": "2030-01-01T00:00:00Z",
    "refresh_token": "pty-test-refresh-token",
    "oidc_issuer": "https://auth.x.ai",
    "oidc_client_id": "b1a00492-073a-47ea-816f-4c329264a828"
  }}
}}"#
        ),
    )
    .expect("seed fake oauth auth.json");
}

/// [`ContentController::env_for_pager`] minus `XAI_API_KEY`, so the entry
/// written by [`seed_fake_oauth`] is the active credential.
pub fn oauth_env_for_pager(content: &ContentController) -> Vec<(String, String)> {
    let mut env = content.env_for_pager();
    env.retain(|(k, _)| k != "XAI_API_KEY");
    env
}

/// Start the mock server with the default `test-model` catalog entry plus a
/// `cursor-model` entry that resolves to a different agent harness.
pub async fn start_dual_agent_type_content() -> ContentController {
    ContentController::start_with_models(vec![
        MockModel::new("test-model"),
        MockModel::with_agent_type("cursor-model", "cursor"),
    ])
    .await
    .expect("start content with dual agent types")
}

/// Drive `/new` until `model` shows on screen. Campaigns apply to **new
/// sessions only** and the pager's settings prefetch is deliberately 2s-capped,
/// so on a loaded runner the first session can legitimately open pre-campaign;
/// each `/new` after the settings fetch lands re-resolves with the campaign.
pub fn wait_for_model_via_new_sessions(h: &mut PtyHarness, model: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if h.contains_text(model) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        let _ = h.inject_keys(b"/new\r");
        h.update(Duration::from_millis(3000));
    }
}

/// Select `model` through the argumentless `/models` picker.
///
/// The picker opens with search input active, so entering the exact model id
/// narrows the catalog and Enter commits the remaining row. This intentionally
/// drives the real user interaction rather than reviving the retired
/// `/model <id>` command contract.
pub fn select_model_from_picker(h: &mut PtyHarness, model: &str, timeout: Duration) {
    inject_keys_paced(h, b"/models", "open models picker");
    h.inject_keys(b"\r").expect("submit /models");
    h.wait_for_text("Model selection", timeout)
        .unwrap_or_else(|_| {
            panic!(
                "model picker did not open while selecting {model:?}\nscreen:\n{}",
                h.screen_contents()
            )
        });

    inject_keys_paced(h, model.as_bytes(), "filter models picker by model id");
    h.wait_for_text(model, timeout).unwrap_or_else(|_| {
        panic!(
            "model {model:?} did not appear in filtered picker\nscreen:\n{}",
            h.screen_contents()
        )
    });
    h.inject_keys(b"\r").expect("select filtered model");

    // Drain until the picker closes (or a short cap expires) so callers can
    // immediately wait on the modal, persisted config, or other post-selection
    // state rather than racing the Enter write still sitting in the PTY queue.
    let close_deadline = Instant::now() + Duration::from_secs(2);
    while h.contains_text("Model selection") && Instant::now() < close_deadline {
        h.update(Duration::from_millis(100));
    }
}

fn inject_keys_paced(h: &mut PtyHarness, keys: &[u8], context: &str) {
    for &byte in keys {
        h.inject_keys(&[byte]).unwrap_or_else(|err| {
            panic!("failed to {context}: {err}");
        });
        h.update(Duration::from_millis(50));
    }
}
