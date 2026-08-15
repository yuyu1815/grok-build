//! `/models` — open the cohesive one-screen model and effort picker.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct ModelsCommand;

impl SlashCommand for ModelsCommand {
    fn name(&self) -> &str {
        "models"
    }

    fn aliases(&self) -> &[&str] {
        &["m"]
    }

    fn description(&self) -> &str {
        "Select model and reasoning effort"
    }

    fn usage(&self) -> &str {
        "/models"
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Error("Usage: /models".into());
        }
        CommandResult::Action(Action::OpenModelsPicker)
    }
}

#[cfg(test)]
mod tests {
    use crate::acp::model_state::ModelState;
    use crate::slash::commands::tests::make_ctx;

    use super::*;

    #[test]
    fn run_opens_picker_without_args_and_rejects_legacy_args() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        for args in ["", "   "] {
            assert!(matches!(
                ModelsCommand.run(&mut ctx, args),
                CommandResult::Action(Action::OpenModelsPicker)
            ));
        }
        assert!(matches!(
            ModelsCommand.run(&mut ctx, "grok"),
            CommandResult::Error(ref message) if message == "Usage: /models"
        ));
    }
}
