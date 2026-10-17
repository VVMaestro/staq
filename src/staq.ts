import { invoke } from "@tauri-apps/api/core";

export const DEFAULT_STAQ_ID = 0;

export interface Task {
  id: number;
  name: string;
  description: string;
  status: "active" | "completed";
  placement: "queue" | "stack";
  staq_id: number | null;
  created_at: string;
}

export interface StaqSnapshot {
  id: number;
  name: string | null;
  description: string | null;
  queue: Task[];
  stack: Task[];
}

export interface TaskPushData {
  name: string;
  description?: string;
}

export interface CreateStaqData {
  name: string;
  description?: string;
}

export function staqLabel(staq: StaqSnapshot): string {
  if (staq.id === DEFAULT_STAQ_ID) return "Основной";
  return staq.name ?? "Без названия";
}

export function staqTaskCount(staq: StaqSnapshot): number {
  return staq.queue.length + staq.stack.length;
}

export function getStaqs(): Promise<StaqSnapshot[]> {
  return invoke<StaqSnapshot[]>("get_staqs");
}

export function getStaq(staqId: number): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("get_staq", { staqId });
}

export function pushToQueue(staqId: number, item: TaskPushData): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_push_to_queue", { item, staqId });
}

export function pushOnStack(staqId: number, item: TaskPushData): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_push_on_stack", { item, staqId });
}

export function popCurrentTask(staqId: number): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_pop", { staqId });
}

export function createStaq(data: CreateStaqData): Promise<StaqSnapshot> {
  return invoke<StaqSnapshot>("staq_create", { data });
}

export function deleteStaq(staqId: number): Promise<void> {
  return invoke<void>("staq_delete", { staqId });
}
