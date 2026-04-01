import React from "react";
import { TaskItem } from "./TaskItem";
import type { TaskSnapshot } from "./types";
import styles from "./TaskList.module.css";

export interface TaskListProps {
  tasks: TaskSnapshot[];
  onSkip?: (taskId: string) => void;
  onRetry?: (taskId: string) => void;
}

export const TaskList: React.FC<TaskListProps> = ({
  tasks,
  onSkip,
  onRetry,
}) => {
  if (tasks.length === 0) {
    return (
      <div className={styles.emptyList}>
        <span>No tasks yet</span>
      </div>
    );
  }

  return (
    <div className={styles.taskList}>
      {tasks.map((task, index) => (
        <TaskItem
          key={task.id}
          task={task}
          index={index}
          onSkip={onSkip}
          onRetry={onRetry}
        />
      ))}
    </div>
  );
};

