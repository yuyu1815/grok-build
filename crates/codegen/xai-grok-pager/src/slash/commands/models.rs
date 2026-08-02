//! `/models` — open the interactive model and reasoning-effort picker.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

/// Open the model picker without changing the current model immediately.
pub struct ModelsCommand;

impl SlashCommand for ModelsCommand {
    fn name(&self) -> &str {
        "models"
    }

    fn description(&self) -> &str {
        "Pick a model"
    }

    fn usage(&self) -> &str {
        "/models"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        if args.trim().is_empty() {
            CommandResult::Action(Action::OpenModelPicker)
        } else {
            CommandResult::Error("Usage: /models".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::slash::commands::tests::make_ctx;

    #[test]
    fn no_args_opens_model_picker() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);

        assert!(matches!(
            ModelsCommand.run(&mut ctx, ""),
            CommandResult::Action(Action::OpenModelPicker)
        ));
    }

    #[test]
    fn args_are_rejected() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);

        assert!(matches!(
            ModelsCommand.run(&mut ctx, "grok-4.5"),
            CommandResult::Error(ref message) if message == "Usage: /models"
        ));
    }
}
