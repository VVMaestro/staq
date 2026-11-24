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
    pub placement: String,
    pub staq_id: Option<i64>,
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
            placement: "queue".into(),
            staq_id: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Staq {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub queue: VecDeque<Task>,
    pub stack: Vec<Task>,
}

impl Staq {
    pub fn new(id: i64, name: Option<String>, desc: Option<String>) -> Self {
        Self {
            id,
            name,
            description: desc,
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    pub fn from_parts(
        id: i64,
        queue: VecDeque<Task>,
        stack: Vec<Task>,
        name: Option<String>,
        desc: Option<String>,
    ) -> Self {
        Self {
            queue,
            stack,
            id,
            name,
            description: desc,
        }
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

    pub fn find_task(&self, id: i64) -> Option<&Task> {
        self.stack
            .iter()
            .chain(self.queue.iter())
            .find(|task| task.id == id)
    }

    pub fn update_task(&mut self, id: i64, name: String, description: String) -> Option<&Task> {
        let task = self
            .stack
            .iter_mut()
            .chain(self.queue.iter_mut())
            .find(|task| task.id == id)?;
        task.name = name;
        task.description = description;
        Some(&*task)
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::{Staq, Task};

    #[test]
    fn new_preserves_id_and_metadata() {
        let staq = Staq::new(42, Some("Work".into()), Some("Work tasks".into()));

        assert_eq!(staq.id, 42);
        assert_eq!(staq.name.as_deref(), Some("Work"));
        assert_eq!(staq.description.as_deref(), Some("Work tasks"));
        assert!(staq.queue.is_empty());
        assert!(staq.stack.is_empty());
    }

    #[test]
    fn new_preserves_optional_metadata() {
        for (name, description) in [
            (None, None),
            (Some("Work".into()), None),
            (None, Some("Work tasks".into())),
        ] {
            let staq = Staq::new(7, name.clone(), description.clone());

            assert_eq!(staq.id, 7);
            assert_eq!(staq.name, name);
            assert_eq!(staq.description, description);
            assert!(staq.queue.is_empty());
            assert!(staq.stack.is_empty());
        }
    }

    #[test]
    fn from_parts_preserves_id_metadata_and_task_order() {
        let queue = VecDeque::from([Task::new(1, "A".into()), Task::new(3, "B".into())]);
        let stack = vec![Task::new(2, "X".into()), Task::new(4, "Y".into())];
        let mut staq = Staq::from_parts(
            42,
            queue,
            stack,
            Some("Work".into()),
            Some("Work tasks".into()),
        );

        assert_eq!(staq.id, 42);
        assert_eq!(staq.name.as_deref(), Some("Work"));
        assert_eq!(staq.description.as_deref(), Some("Work tasks"));
        assert_eq!(
            staq.queue.iter().map(|task| task.id).collect::<Vec<_>>(),
            [1, 3]
        );
        assert_eq!(
            staq.stack.iter().map(|task| task.id).collect::<Vec<_>>(),
            [2, 4]
        );

        for (id, name) in [(4, "Y"), (2, "X"), (1, "A"), (3, "B")] {
            assert_eq!(staq.peek().map(|task| task.id), Some(id));
            let task = staq.pop().expect("a restored task should be available");
            assert_eq!(task.id, id);
            assert_eq!(task.name, name);
        }
        assert!(staq.peek().is_none());
        assert!(staq.pop().is_none());
    }

    #[test]
    fn from_parts_preserves_optional_metadata() {
        for (name, description) in [
            (None, None),
            (Some("Work".into()), None),
            (None, Some("Work tasks".into())),
        ] {
            let staq = Staq::from_parts(
                7,
                VecDeque::new(),
                Vec::new(),
                name.clone(),
                description.clone(),
            );

            assert_eq!(staq.id, 7);
            assert_eq!(staq.name, name);
            assert_eq!(staq.description, description);
            assert!(staq.queue.is_empty());
            assert!(staq.stack.is_empty());
        }
    }

    #[test]
    fn instances_are_isolated() {
        let mut first = Staq::new(10, Some("First".into()), Some("First tasks".into()));
        let mut second = Staq::from_parts(
            20,
            VecDeque::from([Task::new(1, "Second queue".into())]),
            vec![Task::new(2, "Second stack".into())],
            Some("Second".into()),
            Some("Second tasks".into()),
        );
        let second_before = second.serialize();

        first.push(Task::new(1, "First queue".into()));
        first.push_on_stack(Task::new(2, "First stack".into()));
        assert_eq!(first.pop().map(|task| task.id), Some(2));
        first.name = Some("Updated first".into());
        first.description = None;

        assert_eq!(second.serialize(), second_before);
        let first_before = first.serialize();

        assert_eq!(second.pop().map(|task| task.id), Some(2));
        assert_eq!(second.pop().map(|task| task.id), Some(1));
        assert!(second.peek().is_none());
        assert!(second.pop().is_none());
        second.push(Task::new(3, "Another second task".into()));
        second.name = None;
        second.description = Some("Updated second tasks".into());

        assert_eq!(first.serialize(), first_before);
        assert_eq!(first.pop().map(|task| task.id), Some(1));
    }

    #[test]
    fn empty_peek_and_pop_return_none() {
        let mut staq = Staq::new(42, None, None);

        for _ in 0..2 {
            assert!(staq.peek().is_none());
            assert!(staq.pop().is_none());
        }

        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));
        assert_eq!(staq.pop().map(|task| task.id), Some(2));
        assert_eq!(staq.pop().map(|task| task.id), Some(1));

        for _ in 0..2 {
            assert!(staq.peek().is_none());
            assert!(staq.pop().is_none());
        }
        assert!(staq.queue.is_empty());
        assert!(staq.stack.is_empty());
    }

    #[test]
    fn queue_is_fifo() {
        let mut staq = Staq::new(42, None, None);
        staq.push(Task::new(1, "A".into()));
        staq.push(Task::new(3, "B".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("A".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("B".into()));
    }

    #[test]
    fn stack_is_lifo() {
        let mut staq = Staq::new(42, None, None);
        staq.push_on_stack(Task::new(2, "X".into()));
        staq.push_on_stack(Task::new(4, "Y".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("Y".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("X".into()));
    }

    #[test]
    fn stack_has_priority_over_queue() {
        let mut staq = Staq::new(42, None, None);
        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));

        assert_eq!(staq.pop().map(|task| task.name), Some("X".into()));
        assert_eq!(staq.pop().map(|task| task.name), Some("A".into()));
    }

    #[test]
    fn combined_completion_order_is_lifo_then_fifo() {
        let mut staq = Staq::new(42, None, None);
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
        let mut staq = Staq::new(42, None, None);
        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));
        staq.push(Task::new(3, "B".into()));

        let ids = [staq.queue[0].id, staq.stack[0].id, staq.queue[1].id];
        assert_eq!(ids, [1, 2, 3]);
    }

    #[test]
    fn id_counter_is_not_serialized() {
        let mut staq = Staq::new(42, Some("Work".into()), Some("Work tasks".into()));
        staq.push(Task::new(1, "A".into()));
        staq.push_on_stack(Task::new(2, "X".into()));

        let serialized = staq.serialize();
        assert!(!serialized.contains("next_task_id"));

        let value: serde_json::Value =
            serde_json::from_str(&serialized).expect("staq should serialize to valid JSON");
        assert_eq!(value["id"], 42);
        assert_eq!(value["name"], "Work");
        assert_eq!(value["description"], "Work tasks");
        assert_eq!(value["queue"].as_array().unwrap().len(), 1);
        assert_eq!(value["queue"][0]["id"], 1);
        assert_eq!(value["queue"][0]["name"], "A");
        assert_eq!(value["stack"].as_array().unwrap().len(), 1);
        assert_eq!(value["stack"][0]["id"], 2);
        assert_eq!(value["stack"][0]["name"], "X");
    }
}
