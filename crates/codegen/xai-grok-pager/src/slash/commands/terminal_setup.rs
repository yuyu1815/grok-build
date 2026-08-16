//! `/terminal-setup` — diagnose terminal, color/theme, and clipboard setup.
//!
//! Runs the same diagnostics engine used for startup warnings and formats
//! the results as a user-readable message. This gives users an on-demand
//! way to check their environment and see fix instructions.

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use crate::terminal::{TerminalContext, TerminalName};

pub struct TerminalSetupCommand;

fn format_newline_env_line(
    ctx: &TerminalContext,
    shift_enter_unavailable: bool,
    wezterm_kkp_off: bool,
    windows_terminal_needs_alt_enter_binding: bool,
) -> Option<String> {
    if !shift_enter_unavailable || wezterm_kkp_off {
        return None;
    }

    let detail = if ctx.vte_version.is_some() || ctx.brand == TerminalName::Vte {
        match ctx.vte_version.as_deref() {
            Some(v) => format!("VTE {v}; need >= 8200 for Shift+Enter"),
            None => "legacy VTE; need VTE >= 0.82 for Shift+Enter".to_owned(),
        }
    } else if matches!(
        ctx.brand,
        TerminalName::VsCode | TerminalName::Cursor | TerminalName::Windsurf | TerminalName::Zed
    ) {
        format!("{}: xterm.js can't distinguish Shift+Enter", ctx.brand)
    } else if ctx.brand == TerminalName::WindowsTerminal {
        "Windows Terminal + Unix PTY: Shift+Enter == Enter".to_owned()
    } else {
        "no Kitty keyboard protocol; Shift+Enter == Enter".to_owned()
    };
    let setup = if windows_terminal_needs_alt_enter_binding {
        "; verify settings.json binding"
    } else {
        ""
    };
    Some(format!("  newline      Alt+Enter ({detail}{setup})\n"))
}

impl SlashCommand for TerminalSetupCommand {
    fn name(&self) -> &str {
        "terminal-setup"
    }

    fn aliases(&self) -> &[&str] {
        &["terminal-check", "terminal-info"]
    }

    fn description(&self) -> &str {
        "Check terminal, color, and clipboard setup"
    }

    fn usage(&self) -> &str {
        "/terminal-setup"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        let ctx = crate::terminal::terminal_context();
        let query = crate::diagnostics::LiveTmuxQuery;
        let is_control_mode = crate::terminal::detect_tmux_control_mode(ctx);
        let mut warnings = crate::diagnostics::collect_startup_warnings(
            ctx,
            &query,
            is_control_mode,
            _ctx.screen_mode.is_fullscreen(),
        );
        // Live-environment check, kept out of `collect_startup_warnings` so
        // its tests stay hermetic (same pattern as the WezTerm warning below).
        warnings.extend(crate::diagnostics::diagnose_wayland_data_control_live());
        // WezTerm without the Kitty keyboard protocol: surface the fix
        // alongside the other issues. By the time the user runs
        // /terminal-setup the async XTVERSION reply has landed, so this
        // also catches WezTerm over SSH (env brand Unknown, self-report
        // "WezTerm <version>").
        let wezterm_warning = crate::diagnostics::wezterm_kitty_keyboard_warning(
            ctx,
            crate::app::kitty_flags_pushed(),
            crate::terminal::xtversion::detected(),
        );
        let wezterm_kkp_off = wezterm_warning.is_some();
        warnings.extend(wezterm_warning);
        // Windows Terminal + Unix PTY (notably WSL): Shift+Enter needs the
        // Alt+Enter fallback, but WT's stock fullscreen binding consumes it.
        // Keep this in on-demand diagnostics, alongside the equivalent
        // WezTerm setup guidance, rather than adding another startup banner.
        let windows_terminal_alt_enter_warning =
            crate::diagnostics::windows_terminal_alt_enter_warning(
                ctx,
                crate::host::HostOs::current(),
            );
        let windows_terminal_needs_alt_enter_binding = windows_terminal_alt_enter_warning.is_some();
        warnings.extend(windows_terminal_alt_enter_warning);
        // Color not in collect_startup_warnings (noisy on limited terminals).
        let color_level = crate::theme::color_support::get();
        warnings.extend(crate::diagnostics::color_support_warning(
            color_level,
            ctx.brand,
            ctx.is_tmux_backed(),
            &ctx.tmux_config_path(),
        ));
        // SSH wrap recommendation — rendered as its own section below, NOT an
        // issue row: nothing is misconfigured, so it must not put "N issue(s)"
        // on every healthy SSH session. On-demand diagnostics also ignore the
        // `[ui.contextual_hints].ssh_wrap` tip opt-out: that gate (both its
        // user and remote tiers) governs the unprompted session-load tip,
        // while here the user explicitly asked for setup guidance, and an
        // environment report that omits a known improvement would be
        // incomplete.
        let ssh_wrap_recommendation = crate::diagnostics::ssh_wrap_hint(
            ctx.is_ssh,
            crate::clipboard::osc52_sink_active(),
            ctx.is_official_vscode_remote,
        );
        let route = crate::clipboard::clipboard_route();
        let is_ssh = xai_grok_shell::util::clipboard::is_remote_session();
        let container_no_display =
            xai_grok_shell::util::clipboard::is_containerized_without_display();

        let mut out = String::new();

        // -- Environment --
        out.push_str("Environment\n");
        out.push_str(&format!("  terminal     {}\n", ctx.brand));
        if let Some(v) = crate::terminal::xtversion::detected() {
            out.push_str(&format!("  xtversion    {}\n", v));
        }
        out.push_str(&format!("  multiplexer  {}\n", ctx.multiplexer));
        if let Some(ref byobu) = ctx.byobu {
            out.push_str(&format!("  byobu        {}\n", byobu));
        }
        out.push_str(&format!(
            "  ssh          {}\n",
            if is_ssh { "yes" } else { "no" }
        ));
        out.push_str(&crate::diagnostics::format_color_env_line(color_level));
        out.push_str(&crate::diagnostics::format_themes_env_line(color_level));

        let kb = ctx.keyboard_capabilities();
        if kb.modifier_delivery.benefits_from_rescue() || kb.enter_needs_rescue() {
            let rescue = if cfg!(target_os = "macos") {
                "OS rescue active"
            } else {
                "OS rescue unavailable on this platform"
            };
            out.push_str(&format!(
                "  keyboard     {} ({})\n",
                kb.modifier_delivery.label(),
                rescue
            ));
        }

        // Some terminals can't distinguish Shift+Enter from bare Enter at
        // the byte level because the Kitty keyboard protocol isn't
        // negotiated (VTE < 0.82, VS Code's xterm.js, or Windows Terminal
        // with Unix PTY input). Point users at the approved Alt+Enter newline
        // chord. Suppressed when the WezTerm warning fired because its setup
        // path differs; Windows Terminal keeps the row and labels the required
        // binding change, with the exact fix in the issue below.
        if let Some(line) = format_newline_env_line(
            ctx,
            ctx.shift_enter_unavailable(),
            wezterm_kkp_off,
            windows_terminal_needs_alt_enter_binding,
        ) {
            out.push_str(&line);
        }

        // -- Clipboard --
        let display_server = crate::host::DisplayServer::current();
        let is_wayland = display_server == crate::host::DisplayServer::Wayland;
        let clipboard_diagnostics = crate::diagnostics::format_clipboard_diagnostics(
            crate::diagnostics::ClipboardDiagnosticsInput {
                route_native: route.native,
                route_tmux: route.tmux_buffer,
                route_osc52: route.osc52,
                native_tool: xai_grok_shell::util::clipboard::native_tool_name(),
                brand: ctx.brand,
                host_os: crate::host::HostOs::current(),
                display_server,
                is_ssh,
                container_no_display,
                osc52_sink: crate::clipboard::osc52_sink_active(),
                wayland_data_control: is_wayland
                    && xai_grok_shell::util::clipboard::wayland_data_control_supported(),
                wl_copy_available: is_wayland
                    && xai_grok_shell::util::clipboard::native_tool_name() == "wl-copy",
            },
        );
        out.push('\n');
        out.push_str(&clipboard_diagnostics.text);

        // -- Diagnostics --
        if warnings.is_empty() && !clipboard_diagnostics.has_issue {
            out.push_str("\nNo issues found.\n");
        } else if !warnings.is_empty() {
            out.push_str(&format!("\n{} additional issue(s)\n", warnings.len()));
            for w in &warnings {
                out.push_str(&format!("\n  [!] {}\n", w.message));
                match (w.fix.as_deref(), w.config_path.as_deref()) {
                    (Some(fix), Some(path)) => {
                        out.push_str(&format!("      Fix: place `{}` in {}\n", fix, path));
                    }
                    (Some(fix), None) => {
                        out.push_str(&format!("      Fix: run `{}`\n", fix));
                    }
                    _ => {}
                }
                if let Some(note) = w.note.as_deref() {
                    out.push_str(&format!("      Note: {}\n", note));
                }
            }
        }

        // -- Recommendation --
        if let Some(rec) = ssh_wrap_recommendation {
            out.push_str(&format!("\nRecommendation\n\n  {}\n", rec.message));
            if let Some(fix) = rec.fix.as_deref() {
                out.push_str(&format!("      Run: `{}`\n", fix));
            }
            if let Some(note) = rec.note.as_deref() {
                out.push_str(&format!("      Note: {}\n", note));
            }
        }

        CommandResult::Message(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows_terminal_ctx() -> TerminalContext {
        TerminalContext {
            brand: TerminalName::WindowsTerminal,
            env_brand: TerminalName::WindowsTerminal,
            ..Default::default()
        }
    }

    #[test]
    fn windows_terminal_unix_pty_newline_row_requires_binding() {
        let line = format_newline_env_line(&windows_terminal_ctx(), true, false, true)
            .expect("Windows Terminal + Unix PTY must show the fallback row");
        assert_eq!(
            line,
            "  newline      Alt+Enter (Windows Terminal + Unix PTY: Shift+Enter == Enter; verify settings.json binding)\n"
        );
    }

    #[test]
    fn native_windows_terminal_newline_row_stays_hidden() {
        assert!(
            format_newline_env_line(&windows_terminal_ctx(), false, false, false).is_none(),
            "native Windows Terminal keeps Shift+Enter and must not advertise the fallback"
        );
    }

    #[test]
    fn wezterm_warning_suppresses_generic_newline_row() {
        let ctx = TerminalContext {
            brand: TerminalName::WezTerm,
            ..Default::default()
        };
        assert!(format_newline_env_line(&ctx, true, true, false).is_none());
    }
}
