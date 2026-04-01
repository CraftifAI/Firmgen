import React, { useState } from "react";
import type { TaskSnapshot, TaskStatus } from "./types";
import styles from "./TaskItem.module.css";

export interface TaskItemProps {
  task: TaskSnapshot;
  index: number;
  onSkip?: (taskId: string) => void;
  onRetry?: (taskId: string) => void;
}

const STATUS_ICONS: Record<TaskStatus, string> = {
  pending: "⬜",
  in_progress: "🔄",
  completed: "✅",
  skipped: "⏭️",
  failed: "❌",
  blocked: "🚫",
  cancelled: "🚫",
};

const STATUS_COLORS: Record<TaskStatus, string> = {
  pending: "var(--color-text-secondary)",
  in_progress: "var(--color-accent)",
  completed: "var(--color-success)",
  skipped: "var(--color-text-secondary)",
  failed: "var(--color-error)",
  blocked: "var(--color-text-secondary)",
  cancelled: "var(--color-text-secondary)",
};

export const TaskItem: React.FC<TaskItemProps> = ({
  task,
  index,
  onSkip,
  onRetry,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const statusIcon = STATUS_ICONS[task.status] || "❓";
  const statusColor = STATUS_COLORS[task.status] || "var(--color-text)";

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  return (
    <div
      className={`${styles.taskItem} ${styles[task.status]}`}
      onClick={() => setIsExpanded(!isExpanded)}
    >
      <div className={styles.taskMain}>
        <span className={styles.statusIcon}>{statusIcon}</span>
        <div className={styles.taskContent}>
          <div className={styles.taskDescription}>{task.description}</div>
          {task.result_summary && task.status === "completed" && (
            <div className={styles.resultSummary}>{task.result_summary}</div>
          )}
          {task.error_message && task.status === "failed" && (
            <div className={styles.errorMessage}>{task.error_message}</div>
          )}
        </div>
        {task.duration_ms && (
          <span className={styles.duration}>
            {formatDuration(task.duration_ms)}
          </span>
        )}
      </div>

      {isExpanded && (
        <div className={styles.taskDetails}>
          {task.tool_to_call && (
            <div className={styles.detailRow}>
              <span className={styles.detailLabel}>Tool:</span>
              <code className={styles.toolName}>{task.tool_to_call}</code>
            </div>
          )}
          {task.dependencies.length > 0 && (
            <div className={styles.detailRow}>
              <span className={styles.detailLabel}>Depends on:</span>
              <span>{task.dependencies.join(", ")}</span>
            </div>
          )}
          {task.priority !== "normal" && (
            <div className={styles.detailRow}>
              <span className={styles.detailLabel}>Priority:</span>
              <span className={styles[`priority_${task.priority}`]}>
                {task.priority}
              </span>
            </div>
          )}
          <div className={styles.taskActions}>
            {task.can_skip && onSkip && (
              <button
                className={styles.actionButton}
                onClick={(e) => {
                  e.stopPropagation();
                  onSkip(task.id);
                }}
              >
                Skip
              </button>
            )}
            {task.can_retry && onRetry && (
              <button
                className={`${styles.actionButton} ${styles.retryButton}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onRetry(task.id);
                }}
              >
                Retry
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

