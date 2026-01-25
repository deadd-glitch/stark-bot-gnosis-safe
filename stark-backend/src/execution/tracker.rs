use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::{ExecutionTask, TaskMetrics, TaskStatus, TaskType};
use dashmap::DashMap;
use std::sync::Arc;

/// Tracks execution progress for agent tasks
///
/// This service manages the hierarchical task tree for execution tracking,
/// emitting real-time events for the frontend to display progress.
pub struct ExecutionTracker {
    /// Event broadcaster for sending gateway events
    broadcaster: Arc<EventBroadcaster>,
    /// Active tasks indexed by task ID
    tasks: DashMap<String, ExecutionTask>,
    /// Maps channel_id to current execution_id
    channel_executions: DashMap<i64, String>,
}

impl ExecutionTracker {
    /// Create a new ExecutionTracker
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            broadcaster,
            tasks: DashMap::new(),
            channel_executions: DashMap::new(),
        }
    }

    /// Start a new execution for a channel
    ///
    /// Returns the execution ID (which is also the root task ID)
    pub fn start_execution(&self, channel_id: i64, mode: &str) -> String {
        // Create root execution task
        let mut task = ExecutionTask::new(
            channel_id,
            TaskType::Execution,
            format!("Processing request"),
            None,
        ).with_active_form("Processing request".to_string());
        task.start();

        let execution_id = task.id.clone();

        // Track the execution
        self.channel_executions.insert(channel_id, execution_id.clone());
        self.tasks.insert(execution_id.clone(), task.clone());

        // Emit event
        self.broadcaster.broadcast(GatewayEvent::execution_started(
            channel_id,
            &execution_id,
            mode,
        ));
        self.broadcaster.broadcast(GatewayEvent::task_started(&task));

        execution_id
    }

    /// Get the current execution ID for a channel
    pub fn get_execution_id(&self, channel_id: i64) -> Option<String> {
        self.channel_executions.get(&channel_id).map(|v| v.clone())
    }

    /// Add a thinking event to the current execution
    pub fn add_thinking(&self, channel_id: i64, text: &str) {
        if let Some(execution_id) = self.get_execution_id(channel_id) {
            self.broadcaster.broadcast(GatewayEvent::execution_thinking(
                channel_id,
                &execution_id,
                text,
            ));
        }
    }

    /// Start a new task within an execution
    ///
    /// Returns the task ID
    pub fn start_task(
        &self,
        channel_id: i64,
        parent_id: Option<&str>,
        task_type: TaskType,
        description: impl Into<String>,
        active_form: Option<&str>,
    ) -> String {
        let description_str = description.into();
        let mut task = ExecutionTask::new(
            channel_id,
            task_type,
            description_str.clone(),
            parent_id.map(|s| s.to_string()),
        );

        if let Some(form) = active_form {
            task.active_form = Some(form.to_string());
        }

        task.start();
        let task_id = task.id.clone();

        // Update parent's child count
        if let Some(pid) = parent_id {
            if let Some(mut parent) = self.tasks.get_mut(pid) {
                parent.metrics.child_count += 1;
            }
        }

        // Store and emit
        self.tasks.insert(task_id.clone(), task.clone());
        self.broadcaster.broadcast(GatewayEvent::task_started(&task));

        task_id
    }

    /// Start a tool execution task
    ///
    /// Convenience wrapper for starting tool executions
    pub fn start_tool(&self, channel_id: i64, execution_id: &str, tool_name: &str) -> String {
        self.start_task(
            channel_id,
            Some(execution_id),
            TaskType::ToolExecution,
            format!("Using tool: {}", tool_name),
            Some(&format!("Running {}", tool_name)),
        )
    }

    /// Update task metrics
    pub fn update_task_metrics(&self, task_id: &str, metrics: TaskMetrics) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.metrics = metrics.clone();
            self.broadcaster.broadcast(GatewayEvent::task_updated(
                task_id,
                task.channel_id,
                &metrics,
            ));
        }
    }

    /// Add metrics to existing task
    pub fn add_to_task_metrics(&self, task_id: &str, tool_uses: u32, tokens: u32, lines: u32) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.metrics.tool_uses += tool_uses;
            task.metrics.tokens_used += tokens;
            task.metrics.lines_read += lines;

            self.broadcaster.broadcast(GatewayEvent::task_updated(
                task_id,
                task.channel_id,
                &task.metrics.clone(),
            ));
        }
    }

    /// Complete a task successfully
    pub fn complete_task(&self, task_id: &str) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.complete();
            self.broadcaster.broadcast(GatewayEvent::task_completed(
                task_id,
                task.channel_id,
                "completed",
                &task.metrics,
            ));
        }
    }

    /// Complete a task with an error
    pub fn complete_task_with_error(&self, task_id: &str, error: &str) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.complete_with_error(error);
            self.broadcaster.broadcast(GatewayEvent::task_completed(
                task_id,
                task.channel_id,
                &format!("error: {}", error),
                &task.metrics,
            ));
        }
    }

    /// Complete an entire execution
    ///
    /// Aggregates metrics from all child tasks
    pub fn complete_execution(&self, channel_id: i64) {
        if let Some((_, execution_id)) = self.channel_executions.remove(&channel_id) {
            // Aggregate metrics from all tasks in this execution
            let mut total_metrics = TaskMetrics::default();
            let mut task_ids_to_remove = Vec::new();

            for entry in self.tasks.iter() {
                let task = entry.value();
                if task.channel_id == channel_id {
                    total_metrics.tool_uses += task.metrics.tool_uses;
                    total_metrics.tokens_used += task.metrics.tokens_used;
                    total_metrics.lines_read += task.metrics.lines_read;
                    task_ids_to_remove.push(entry.key().clone());
                }
            }

            // Complete the root task
            if let Some(mut root_task) = self.tasks.get_mut(&execution_id) {
                root_task.complete();
                total_metrics.duration_ms = root_task.metrics.duration_ms;
            }

            // Emit completion event
            self.broadcaster.broadcast(GatewayEvent::execution_completed(
                channel_id,
                &execution_id,
                &total_metrics,
            ));

            // Clean up tasks for this execution
            for task_id in task_ids_to_remove {
                self.tasks.remove(&task_id);
            }
        }
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: &str) -> Option<ExecutionTask> {
        self.tasks.get(task_id).map(|t| t.clone())
    }

    /// Get all tasks for a channel
    pub fn get_channel_tasks(&self, channel_id: i64) -> Vec<ExecutionTask> {
        self.tasks
            .iter()
            .filter(|entry| entry.value().channel_id == channel_id)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tracker() -> ExecutionTracker {
        let broadcaster = Arc::new(EventBroadcaster::new());
        ExecutionTracker::new(broadcaster)
    }

    #[test]
    fn test_execution_lifecycle() {
        let tracker = create_test_tracker();

        // Start execution
        let execution_id = tracker.start_execution(1, "execute");
        assert!(!execution_id.is_empty());
        assert!(tracker.get_execution_id(1).is_some());

        // Start a tool task
        let tool_id = tracker.start_tool(1, &execution_id, "web_search");
        assert!(!tool_id.is_empty());

        // Complete the tool
        tracker.complete_task(&tool_id);
        let task = tracker.get_task(&tool_id).unwrap();
        assert!(matches!(task.status, TaskStatus::Completed));

        // Complete execution
        tracker.complete_execution(1);
        assert!(tracker.get_execution_id(1).is_none());
    }

    #[test]
    fn test_metrics_aggregation() {
        let tracker = create_test_tracker();

        let execution_id = tracker.start_execution(1, "execute");

        // Start multiple tools
        let tool1 = tracker.start_tool(1, &execution_id, "tool1");
        tracker.add_to_task_metrics(&tool1, 1, 100, 10);
        tracker.complete_task(&tool1);

        let tool2 = tracker.start_tool(1, &execution_id, "tool2");
        tracker.add_to_task_metrics(&tool2, 1, 200, 20);
        tracker.complete_task(&tool2);

        // Check that metrics are tracked
        let task1 = tracker.get_task(&tool1).unwrap();
        assert_eq!(task1.metrics.tool_uses, 1);
        assert_eq!(task1.metrics.tokens_used, 100);

        let task2 = tracker.get_task(&tool2).unwrap();
        assert_eq!(task2.metrics.tool_uses, 1);
        assert_eq!(task2.metrics.tokens_used, 200);
    }
}
