import React from "react";
import { defaultSize, type AnimationSize } from "./types";
import styles from "./LogoAnimation.module.css";

export type LogoAnimationProps = {
  size?: AnimationSize;
  isWaiting: boolean;
  isStreaming: boolean;
  style?: React.CSSProperties;
};

const sizeMap: Record<AnimationSize, string> = {
  "1": "6px",
  "2": "8px",
  "3": "10px",
  "4": "12px",
  "5": "14px",
  "6": "16px",
  "7": "20px",
  "8": "24px",
};

export const LogoAnimation: React.FC<LogoAnimationProps> = ({
  size = defaultSize,
  isWaiting,
  isStreaming,
  style,
}) => {
  if (!isStreaming && !isWaiting) return null;

  return (
    <div
      className={styles.spinner}
      style={{
        width: sizeMap[size],
        height: sizeMap[size],
        ...style,
      }}
      aria-label="Loading"
    />
  );
};
