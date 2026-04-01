export interface WorkflowProgress {
  total: number;
  completed: number;
  in_progress: number;
  pending: number;
  skipped: number;
  failed: number;
  percentage: number;
}

export interface TaskSnapshot {
  id: string;
  description: string;
  status: TaskStatus;
  tool_to_call: string | null;
  result_summary: string | null;
  error_message: string | null;
  started_at: string | null;
  completed_at: string | null;
  duration_ms: number | null;
  can_skip: boolean;
  can_retry: boolean;
  dependencies: string[];
  priority: string;
}

export type TaskStatus =
  | "pending"
  | "in_progress"
  | "completed"
  | "skipped"
  | "failed"
  | "blocked"
  | "cancelled";

export interface WorkflowSnapshot {
  has_active_workflow: boolean;
  workflow_id: string | null;
  workflow_name: string | null;
  workflow_type: string | null;
  device_context: string | null;
  progress: WorkflowProgress;
  tasks: TaskSnapshot[];
  can_pause: boolean;
  can_resume: boolean;
  is_paused: boolean;
}

export interface WorkflowEvent {
  event: string;
  workflow_id?: string;
  task_id?: string;
  success?: boolean;
}

