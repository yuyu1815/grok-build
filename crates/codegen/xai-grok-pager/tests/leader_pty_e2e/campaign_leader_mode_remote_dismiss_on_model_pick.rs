// Per-test-case module for the `leader_pty_e2e` integration test crate.
#[allow(unused_imports)]
use super::common::*;

/// **Leader mode: a `/models` picker choice in the TUI dismisses a remote campaign.**
///
/// The dismiss chokepoint (`persist_user_choice`) runs in the **TUI process**,
/// but in leader mode no in-process agent ever seeds the TUI's remote campaign
/// cache — only `app::run`'s own seed makes a remote campaign visible to
/// `resolve_dismissable_campaigns`. Without that seed this test times out in
/// the dismiss phase: the pick persists but no dismissal is recorded, and the
/// leader re-nudges every new session over the user's explicit choice.
///
/// The TUI's settings prefetch is deliberately 2s-capped, so on a loaded
/// runner a spawn can miss the fetch (unseeded cache — the documented
/// transient leader-mode divergence). The test retries with fresh TUI spawns
/// (same leader) until a pick lands the dismissal, then proves it sticks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "PTY e2e; run with cargo test -p xai-grok-pager --test leader_pty_e2e -- --ignored --test-threads=1"]
async fn campaign_leader_mode_remote_dismiss_on_model_pick() {
    const INITIAL_CONFIG_MODEL: &str = "initial-config-model";
    const CAMPAIGN_MODEL: &str = "campaign-model";
    const PICKED_MODEL: &str = "picked-model";
    const CAMPAIGN_ID: &str = "e2e-leader-remote-nudge";

    let content = ContentController::start_with_models(vec![
        MockModel::new(INITIAL_CONFIG_MODEL),
        MockModel::new(CAMPAIGN_MODEL),
        MockModel::new(PICKED_MODEL),
    ])
    .await
    .expect("start content with three models");

    // Serve the campaign from the settings endpoint (restating `allow_access`,
    // which the preset otherwise provides).
    content.server().set_settings(json!({
        "allow_access": true,
        "campaigns": [
            { "id": CAMPAIGN_ID, "models": { "default": CAMPAIGN_MODEL } }
        ]
    }));

    // Seed config.toml with the user's own default model; a fixed leader
    // socket under the shared GROK_HOME so every spawn elects/attaches to the
    // same leader (mirrors `LeaderCluster`).
    let grok_home = content.home().join(".grok");
    std::fs::create_dir_all(&grok_home).expect("create GROK_HOME");
    std::fs::write(
        grok_home.join("config.toml"),
        format!("[models]\ndefault = \"{INITIAL_CONFIG_MODEL}\"\n"),
    )
    .expect("write config.toml");
    let socket = grok_home.join("leader-e2e.sock");
    let socket = socket.to_str().expect("socket path is utf-8").to_owned();

    // Session (OAuth) auth, not the harness's default XAI_API_KEY: the
    // settings fetch requires `auth_manager.auth()` — in ApiKey/BYOK mode the
    // pager never requests `/v1/settings`, so a remote campaign would be
    // structurally unreachable (see `spawn_polling_session`'s doc).
    seed_fake_oauth(&content, "pty-campaign-leader");
    let binary = pager_binary().expect("resolve pager binary");
    let env = oauth_env_for_pager(&content);
    let spawn = || -> PtyHarness {
        let env_refs: Vec<(&str, &str)> =
            env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        PtyHarness::new(
            &binary,
            DEFAULT_ROWS,
            DEFAULT_COLS,
            &["--leader", "--leader-socket", &socket],
            &env_refs,
        )
        .expect("spawn leader-mode pager")
    };
    let state_path = grok_home.join("campaigns_state.json");
    let dismissed = |state_path: &std::path::Path| {
        std::fs::read_to_string(state_path)
            .map(|s| s.contains(CAMPAIGN_ID))
            .unwrap_or(false)
    };

    // ── Phase 1+2: nudge on a new session; a pick records the dismissal in
    // the TUI process. Retries fresh TUI spawns (same leader) so a missed
    // 2s prefetch window on a loaded runner can't wedge the test.
    let mut recorded = false;
    'attempts: for attempt in 0..3 {
        let mut h = spawn();
        h.wait_for_text(WELCOME_SCREEN_SENTINEL, LEADER_TIMEOUT)
            .unwrap_or_else(|_| {
                panic!(
                    "leader-mode welcome never rendered (attempt {attempt})\nscreen:\n{}",
                    h.screen_contents()
                )
            });
        if !wait_for_model_via_new_sessions(&mut h, CAMPAIGN_MODEL, Duration::from_secs(60)) {
            // Campaign never applied on this spawn; try a fresh TUI.
            h.quit().expect("clean quit");
            continue;
        }
        select_model_from_picker(&mut h, PICKED_MODEL, Duration::from_secs(15));
        let config_path = grok_home.join("config.toml");
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            h.update(Duration::from_millis(200));
            let picked_persisted = std::fs::read_to_string(&config_path)
                .map(|s| s.contains(&format!("default = \"{PICKED_MODEL}\"")))
                .unwrap_or(false);
            if dismissed(&state_path) && picked_persisted {
                recorded = true;
                h.quit().expect("clean quit");
                break 'attempts;
            }
        }
        // Dismissal is written before config persistence. Require both before
        // accepting an attempt; a prefetch miss can still leave dismissal
        // absent, in which case retry with a fresh TUI against the same leader.
        h.quit().expect("clean quit");
    }
    assert!(
        recorded,
        "leader-mode TUI must record the remote campaign dismissal in {state_path:?}"
    );

    // ── Phase 3: the dismissal is durable, the explicit pick is persisted,
    // and a reboot against the same leader/settings keeps the pick winning. ──
    let config = std::fs::read_to_string(grok_home.join("config.toml")).expect("read config.toml");
    assert!(
        config.contains(&format!("default = \"{PICKED_MODEL}\"")),
        "the explicit pick must be persisted to config.toml:\n{config}"
    );
    assert!(
        !config.contains(CAMPAIGN_MODEL),
        "the campaign value must never be written to config.toml:\n{config}"
    );
    assert!(
        !config.contains(INITIAL_CONFIG_MODEL),
        "the stale initial default must be replaced by the explicit pick:\n{config}"
    );
    assert!(
        dismissed(&state_path),
        "the dismissal must survive on disk after the client exits"
    );

    let mut h = spawn();
    h.wait_for_text(WELCOME_SCREEN_SENTINEL, LEADER_TIMEOUT)
        .expect("leader-mode welcome renders after reboot");
    h.wait_for_text(PICKED_MODEL, Duration::from_secs(30))
        .unwrap_or_else(|_| {
            panic!(
                "after reboot the explicitly picked model must win\nscreen:\n{}",
                h.screen_contents()
            )
        });
    let _ = h.inject_keys(b"/new\r");
    h.update(Duration::from_millis(4000));
    h.wait_for_text(PICKED_MODEL, Duration::from_secs(20))
        .expect("picked model after leader post-fetch /new");
    assert!(
        !h.contains_text(CAMPAIGN_MODEL),
        "the dismissed campaign must not re-apply after leader reboot\nscreen:\n{}",
        h.screen_contents()
    );
    assert!(
        !h.contains_text(INITIAL_CONFIG_MODEL),
        "the stale initial config model must not beat the explicit pick\nscreen:\n{}",
        h.screen_contents()
    );
    h.quit().expect("clean quit after reboot assertion");
}
