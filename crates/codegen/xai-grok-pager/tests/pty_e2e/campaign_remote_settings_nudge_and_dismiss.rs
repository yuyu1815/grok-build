// Per-test-case module for the `pty_e2e` integration test crate.
#[allow(unused_imports)]
use super::common::*;

/// **Campaign nudge via the real remote path** — the production source
/// (`GET /v1/settings` → `RemoteSettings.campaigns` → process cache seed →
/// apply/dismiss), unlike the sibling test which injects the campaign through
/// `GROK_CAMPAIGNS_OVERRIDE`.
///
/// - boot with a `[models].default` in config.toml plus a **server-served**
///   campaign nudging a *different* model → a (possibly not first — see
///   [`wait_for_model_via_new_sessions`]) new session opens on the
///   **campaign** model;
/// - pick a third, explicit model via the `/models` picker → the remote campaign
///   id is recorded dismissed in `campaigns_state.json` and the picked model is
///   persisted to `config.toml`;
/// - reboot against the *same* server settings → the **picked** model wins and
///   stays winning across `/new`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "PTY e2e; run with cargo test -p xai-grok-pager --test pty_e2e -- --ignored"]
async fn campaign_remote_settings_nudge_and_dismiss() {
    const INITIAL_CONFIG_MODEL: &str = "initial-config-model";
    const CAMPAIGN_MODEL: &str = "campaign-model";
    const PICKED_MODEL: &str = "picked-model";
    const CAMPAIGN_ID: &str = "e2e-remote-nudge";

    let content = ContentController::start_with_models(vec![
        MockModel::new(INITIAL_CONFIG_MODEL),
        MockModel::new(CAMPAIGN_MODEL),
        MockModel::new(PICKED_MODEL),
    ])
    .await
    .expect("start content with three models");

    // Serve the campaign from the settings endpoint (replaces the preset, so
    // `allow_access` must be restated or the pager parks on the upsell screen).
    content.server().set_settings(json!({
        "allow_access": true,
        "campaigns": [
            { "id": CAMPAIGN_ID, "models": { "default": CAMPAIGN_MODEL } }
        ]
    }));

    // Seed config.toml with the user's own default model.
    let grok_home = content.home().join(".grok");
    std::fs::create_dir_all(&grok_home).expect("create GROK_HOME");
    std::fs::write(
        grok_home.join("config.toml"),
        format!("[models]\ndefault = \"{INITIAL_CONFIG_MODEL}\"\n"),
    )
    .expect("write config.toml");

    // Session (OAuth) auth, not the harness's default XAI_API_KEY: the
    // settings fetch requires `auth_manager.auth()` — in ApiKey/BYOK mode the
    // pager never requests `/v1/settings`, so a remote campaign would be
    // structurally unreachable (see `spawn_polling_session`'s doc).
    seed_fake_oauth(&content, "pty-campaign-remote");
    let binary = pager_binary().expect("resolve pager binary");
    let env = oauth_env_for_pager(&content);
    let spawn = || -> PtyHarness {
        let env_refs: Vec<(&str, &str)> =
            env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        PtyHarness::new(&binary, DEFAULT_ROWS, DEFAULT_COLS, &[], &env_refs).expect("spawn pager")
    };

    // ── Phase 1+2: the campaign applies to a new session; a pick dismisses. ──
    {
        let mut h = spawn();
        h.wait_for_text(WELCOME_SCREEN_SENTINEL, WELCOME_TIMEOUT)
            .expect("welcome renders");
        assert!(
            wait_for_model_via_new_sessions(&mut h, CAMPAIGN_MODEL, Duration::from_secs(60)),
            "a new session should open on the remote campaign model\nscreen:\n{}",
            h.screen_contents()
        );
        assert!(
            !h.contains_text("panicked"),
            "pager panicked\n{}",
            h.screen_contents()
        );

        // Explicit picker choice of a third model → persists that choice and
        // dismisses the campaign. Dismissal is intentionally written before
        // config persistence, so wait for both durable postconditions.
        select_model_from_picker(&mut h, PICKED_MODEL, Duration::from_secs(15));

        let state_path = grok_home.join("campaigns_state.json");
        let config_path = grok_home.join("config.toml");
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            h.update(Duration::from_millis(200));
            let dismissed = std::fs::read_to_string(&state_path)
                .map(|s| s.contains(CAMPAIGN_ID))
                .unwrap_or(false);
            let picked_persisted = std::fs::read_to_string(&config_path)
                .map(|s| s.contains(&format!("default = \"{PICKED_MODEL}\"")))
                .unwrap_or(false);
            if dismissed && picked_persisted {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "remote campaign dismissal and picked model persistence must both land\nstate: {state_path:?}\nconfig: {config_path:?}\nscreen:\n{}",
                h.screen_contents()
            );
        }
        h.quit().expect("clean quit");
    }

    // ── Phase 3: reboot against the SAME settings → the picked model wins. ──
    {
        let mut h = spawn();
        h.wait_for_text(WELCOME_SCREEN_SENTINEL, WELCOME_TIMEOUT)
            .expect("welcome renders after reboot");
        h.wait_for_text(PICKED_MODEL, Duration::from_secs(20))
            .unwrap_or_else(|_| {
                panic!(
                    "after dismissal the explicitly picked model must show\nscreen:\n{}",
                    h.screen_contents()
                )
            });
        // Give the settings fetch time to land, then prove a fresh session
        // still resolves to the picked model (dismissed campaigns never
        // re-apply, even once the remote campaign is in the cache).
        let _ = h.inject_keys(b"/new\r");
        h.update(Duration::from_millis(4000));
        h.wait_for_text(PICKED_MODEL, Duration::from_secs(10))
            .expect("picked model after post-fetch /new");
        assert!(
            !h.contains_text(CAMPAIGN_MODEL),
            "a dismissed remote campaign must not re-nudge the model\nscreen:\n{}",
            h.screen_contents()
        );
        assert!(
            !h.contains_text(INITIAL_CONFIG_MODEL),
            "the stale initial config model must not beat the explicit pick\nscreen:\n{}",
            h.screen_contents()
        );
        h.quit().expect("clean quit");
    }
}
