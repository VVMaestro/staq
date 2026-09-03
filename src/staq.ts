import { invoke } from "@tauri-apps/api/core";

export interface Task {
  id: number;
  name: string;
  description: string;
  status: string;
  created_at: string;
}

export interface StaqSnapshot {
  queue: Task[];
  stack: Task[];
}

export function getStaq(): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("get_staq");
}

export function pushToQueue(item: string): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_push_to_queue", { item });
}

export function pushOnStack(item: string): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_push_on_stack", { item });
}

export function popCurrentTask(): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_pop");
}
