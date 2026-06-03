//! Tool call execution

use super::AgentRuntime;
use anycode_core::prelude::*;

impl AgentRuntime {
    pub(super) async fn execute_tool_call(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        working_directory: &str,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, CoreError> {
        let tools = self.tools.read().await;
        self.run_tool_invocation_pipeline(&tools, task_id, agent_type, working_directory, tool_call)
            .await
    }
}
