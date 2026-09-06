use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::Serialize;

pub enum TaskStatus {
    Active,
    Completed,
}

#[derive(Clone, Serialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn new(id: i64, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            status: "active".into(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Staq {
    pub queue: VecDeque<Task>,
    pub stack: Vec<Task>,
}

impl Staq {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    pub fn from_parts(queue: VecDeque<Task>, stack: Vec<Task>) -> Self {
        Self { queue, stack }
    }

    pub fn push(&mut self, task: Task) {
        self.queue.push_back(task);
    }

    pub fn push_on_stack(&mut self, task: Task) {
        self.stack.push(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        if !self.stack.is_empty() {
            self.stack.pop()
        } else if !self.queue.is_empty() {
            self.queue.pop_front()
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&Task> {
        if !self.stack.is_empty() {
            self.stack.last()
        } else if !self.queue.is_empty() {
            self.queue.front()
        } else {
            None
        }
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{Staq, Task};

    #[test]
    fn queue_is_fifo() {
        let mut staq = Staq::new();
        staq.push(Task::new(1, "A".into()));
        staq.push(Task::new(3, "B".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("A".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("B".into()));
    }

    #[test]
    fn stack_is_lifo() {
        let mut staq = Staq::new();
        staq.push_on_stack(Task::new(2, "X".into()));
        staq.push_on_stack(Task::new(4, "Y".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("Y".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("X".into()));
    }

    #[test]
    fn stack_has_priority_over_queue() {
        let mut staq = Staq::new();
        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("X".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("A".into()));
    }

    #[test]
    fn combined_completion_order_is_lifo_then_fifo() {
        let mut staq = Staq::new();
        staq.push(Task::new(1, "A".into()));
        staq.push(Task::new(3, "B".into()));
        staq.push_on_stack(Task::new(2, "X".into()));
        staq.push_on_stack(Task::new(4, "Y".into()));

        let completed: Vec<_> = (0..4)
            .map(|_| staq.pop().expect("a task should be available").name)
            .collect();

        assert_eq!(completed, ["Y", "X", "A", "B"]);
    }

    #[test]
    fn preserves_supplied_ids() {
        let mut staq = Staq::new();
        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));
        staq.push(Task::new(3, "B".into()));

        let ids = [staq.queue[0].id, staq.stack[0].id, staq.queue[1].id];
        assert_eq!(ids, [1, 2, 3]);
    }

    #[test]
    fn id_counter_is_not_serialized() {
        let mut staq = Staq::new();
        staq.push(Task::new(1, "A".into()));

        let serialized = staq.serialize();
        assert!(!serialized.contains("next_task_id"));
    }
}
