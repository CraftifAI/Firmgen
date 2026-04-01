import React from "react";
import { TaskList } from "./TaskList";
import type { WorkflowSnapshot, TaskSnapshot } from "./types";
import styles from "./WorkflowPanel.module.css";

export interface WorkflowPanelProps {
  snapshot: WorkflowSnapshot;
  onPause?: () => void;
  onResume?: () => void;
  onCancel?: () => void;
  onSkipTask?: (taskId: string) => void;
  onRetryTask?: (taskId: string) => void;
  isLoading?: boolean;
}

export const WorkflowPanel: React.FC<WorkflowPanelProps> = ({
  snapshot,
  onPause,
  onResume,
  onCancel,
  onSkipTask,
  onRetryTask,
  isLoading = false,
}) => {
  if (!snapshot.has_active_workflow) {
    return (
      <div className={styles.workflowPanel}>
        <div className={styles.emptyState}>
          <div className={styles.emptyIcon}>📋</div>
          <div className={styles.emptyTitle}>No Active Workflow</div>
          <div className={styles.emptyDescription}>
            Start a complex task and the agent will create a workflow to track
            progress.
          </div>
        </div>
      </div>
    );
  }

  const { progress } = snapshot;

  return (
    <div className={styles.workflowPanel}>
      <div className={styles.header}>
        <div>
          <h3 className={styles.title}>
            {snapshot.workflow_name || "Workflow"}
          </h3>
          <div className={styles.subtitle}>
            {snapshot.device_context && (
              <span className={styles.deviceBadge}>
                {snapshot.device_context.toUpperCase()}
              </span>
            )}
            {snapshot.is_paused && (
              <span className={styles.pausedBadge}>⏸️ Paused</span>
            )}
          </div>
        </div>
      </div>

      <div className={styles.progressSection}>
        <div className={styles.progressBar}>
          <div
            className={styles.progressFill}
            style={{ width: `${progress.percentage}%` }}
          />
        </div>
        <div className={styles.progressText}>
          <span>
            {progress.completed} of {progress.total} tasks
          </span>
          <span>{Math.round(progress.percentage)}%</span>
        </div>
      </div>

      <div className={styles.taskListContainer}>
        <TaskList
          tasks={snapshot.tasks}
          onSkip={onSkipTask}
          onRetry={onRetryTask}
        />
      </div>

      <div className={styles.controls}>
        {snapshot.can_pause && (
          <button
            className={styles.controlButton}
            onClick={onPause}
            disabled={isLoading}
          >
            ⏸️ Pause
          </button>
        )}
        {snapshot.can_resume && (
          <button
            className={`${styles.controlButton} ${styles.primary}`}
            onClick={onResume}
            disabled={isLoading}
          >
            ▶️ Resume
          </button>
        )}
        <button
          className={`${styles.controlButton} ${styles.danger}`}
          onClick={onCancel}
          disabled={isLoading}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};

