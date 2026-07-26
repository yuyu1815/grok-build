//! Tool definition types for the model API.
//!
//! These types represent tool schemas sent to the model when tools are
//! advertised for a turn.

pub use xai_grok_sampling_types::definition::{FunctionTool, ToolDefinition, ToolType};

#[cfg(test)]
mod tests {
    use super::{ToolDefinition as ToolsDefinition, ToolType as ToolsToolType};
    use serde_json::json;
    use xai_grok_sampling_types::{
        ToolDefinition as SamplingRootDefinition, ToolType as SamplingRootToolType,
        definition::{ToolDefinition as SamplingDefinition, ToolType as SamplingToolType},
        types::{ToolDefinition as SamplingTypesDefinition, ToolType as SamplingTypesToolType},
    };

    fn accepts_sampling_definition(_: SamplingDefinition) {}
    fn accepts_tools_definition(_: ToolsDefinition) {}

    #[test]
    fn public_paths_resolve_to_the_same_types() {
        let definition = SamplingRootDefinition::function("example", Some("desc"), json!({}));
        accepts_sampling_definition(definition.clone());
        accepts_tools_definition(definition.clone());
        let _: SamplingTypesDefinition = definition.clone();
        let _: ToolsDefinition = definition;

        let _: SamplingRootToolType = SamplingToolType::Function;
        let _: SamplingTypesToolType = SamplingRootToolType::Function;
        let _: ToolsToolType = SamplingTypesToolType::Function;
    }

    #[test]
    fn tool_definition_json_shape_round_trips_with_optional_description() {
        for (description, expected_function) in [
            (
                Some("desc"),
                json!({
                    "name": "example",
                    "description": "desc",
                    "parameters": {}
                }),
            ),
            (None, json!({"name": "example", "parameters": {}})),
        ] {
            let definition = SamplingDefinition::function("example", description, json!({}));
            let value = serde_json::to_value(&definition).unwrap();
            assert_eq!(
                value,
                json!({"type": "function", "function": expected_function})
            );
            let round_trip: ToolsDefinition = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(round_trip).unwrap(), value);
        }

        assert_eq!(
            serde_json::to_value(SamplingToolType::Function).unwrap(),
            json!("function")
        );
    }
}
