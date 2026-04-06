//! Agent 路由类型标识。

use serde::{Deserialize, Serialize};

/// Agent 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentType(String);

impl AgentType {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
