//! Workflow Context - Tracks accumulated state during workflow execution

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Context accumulated during workflow execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Facts accumulated during workflow (key-value pairs)
    pub facts: HashMap<String, Value>,
    
    /// Decisions made during workflow
    pub decisions: Vec<WorkflowDecision>,
    
    /// Errors encountered (for reference)
    pub errors_encountered: Vec<WorkflowError>,
    
    /// User interventions
    pub user_interventions: Vec<UserIntervention>,
    
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}

/// A decision made during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDecision {
    pub task_id: String,
    pub decision: String,
    pub reason: String,
    pub timestamp: String,
    pub alternatives_considered: Vec<String>,
}

/// An error encountered during workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowError {
    pub task_id: String,
    pub error_type: String,
    pub message: String,
    pub timestamp: String,
    pub recovered: bool,
    pub recovery_action: Option<String>,
}

/// User intervention during workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntervention {
    pub intervention_type: InterventionType,
    pub task_id: Option<String>,
    pub reason: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionType {
    Pause,
    Resume,
    Skip,
    Retry,
    Reorder,
    Cancel,
    AddTask,
    ModifyTask,
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set a fact in the context
    pub fn set_fact(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.facts.insert(key.into(), value.into());
    }
    
    /// Get a fact from the context
    pub fn get_fact(&self, key: &str) -> Option<&Value> {
        self.facts.get(key)
    }
    
    /// Get a fact as a string
    pub fn get_fact_string(&self, key: &str) -> Option<String> {
        self.facts.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }
    
    /// Record a decision
    pub fn record_decision(&mut self, task_id: impl Into<String>, decision: impl Into<String>, reason: impl Into<String>) {
        self.decisions.push(WorkflowDecision {
            task_id: task_id.into(),
            decision: decision.into(),
            reason: reason.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            alternatives_considered: vec![],
        });
    }
    
    /// Record an error
    pub fn record_error(&mut self, task_id: impl Into<String>, error_type: impl Into<String>, message: impl Into<String>) {
        self.errors_encountered.push(WorkflowError {
            task_id: task_id.into(),
            error_type: error_type.into(),
            message: message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            recovered: false,
            recovery_action: None,
        });
    }
    
    /// Mark an error as recovered
    pub fn mark_error_recovered(&mut self, task_id: &str, recovery_action: impl Into<String>) {
        if let Some(error) = self.errors_encountered.iter_mut()
            .rev()
            .find(|e| e.task_id == task_id && !e.recovered)
        {
            error.recovered = true;
            error.recovery_action = Some(recovery_action.into());
        }
    }
    
    /// Record user intervention
    pub fn record_intervention(&mut self, intervention_type: InterventionType, task_id: Option<String>, reason: Option<String>) {
        self.user_interventions.push(UserIntervention {
            intervention_type,
            task_id,
            reason,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
    
    /// Get a summary of the context (for LLM consumption)
    pub fn get_summary(&self, max_tokens: usize) -> String {
        let mut summary = String::new();
        let mut estimated_tokens = 0;
        
        // Add key facts
        if !self.facts.is_empty() {
            summary.push_str("FACTS: ");
            for (key, value) in &self.facts {
                let fact_str = format!("{}={}, ", key, value);
                if estimated_tokens + fact_str.len() / 4 > max_tokens {
                    break;
                }
                summary.push_str(&fact_str);
                estimated_tokens += fact_str.len() / 4;
            }
            summary.push('\n');
        }
        
        // Add recent errors
        if !self.errors_encountered.is_empty() {
            let recent_errors: Vec<_> = self.errors_encountered.iter()
                .filter(|e| !e.recovered)
                .take(3)
                .collect();
            
            if !recent_errors.is_empty() {
                summary.push_str("ERRORS: ");
                for error in recent_errors {
                    let error_str = format!("[{}] {}, ", error.task_id, error.message);
                    if estimated_tokens + error_str.len() / 4 > max_tokens {
                        break;
                    }
                    summary.push_str(&error_str);
                    estimated_tokens += error_str.len() / 4;
                }
                summary.push('\n');
            }
        }
        
        summary
    }
    
    /// Merge another context into this one
    pub fn merge(&mut self, other: WorkflowContext) {
        self.facts.extend(other.facts);
        self.decisions.extend(other.decisions);
        self.errors_encountered.extend(other.errors_encountered);
        self.user_interventions.extend(other.user_interventions);
        self.metadata.extend(other.metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_facts() {
        let mut ctx = WorkflowContext::new();
        ctx.set_fact("project_name", "my_project");
        ctx.set_fact("build_config", "release");
        
        assert_eq!(ctx.get_fact_string("project_name"), Some("my_project".to_string()));
    }
    
    #[test]
    fn test_context_errors() {
        let mut ctx = WorkflowContext::new();
        ctx.record_error("task_1", "build_error", "Missing header file");
        
        assert_eq!(ctx.errors_encountered.len(), 1);
        assert!(!ctx.errors_encountered[0].recovered);
        
        ctx.mark_error_recovered("task_1", "Added include path");
        assert!(ctx.errors_encountered[0].recovered);
    }
}

