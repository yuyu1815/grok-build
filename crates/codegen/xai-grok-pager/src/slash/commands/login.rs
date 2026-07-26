//! `/login` -- log in or re-authenticate with your account.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct LoginCommand;

impl SlashCommand for LoginCommand {
    fn name(&self) -> &str {
        "login"
    }

    fn description(&self) -> &str {
        "Log in or re-authenticate with your account"
    }

    fn usage(&self) -> &str {
        "/login grok | /login provider grok"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let accepted = matches!(
            args.split_whitespace().collect::<Vec<_>>().as_slice(),
            [] | ["grok"] | ["provider", "grok"]
        );
        if accepted {
            CommandResult::Action(Action::Login)
        } else {
            CommandResult::Error("Usage: /login, /login grok, or /login provider grok".to_string())
        }
    }
}
