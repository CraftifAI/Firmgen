import React from "react";
import { ProgressBar } from "./ProgressBar";
import type { ProgressBarProps } from "./ProgressBar";

interface ErrorBoundaryState {
  hasError: boolean;
}

class ProgressBarErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ErrorBoundaryState
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error) {
    console.error("[ProgressBar] render error caught by boundary:", error);
  }

  render() {
    if (this.state.hasError) return null;
    return this.props.children;
  }
}

export const ProgressBarSafe: React.FC<ProgressBarProps> = (props) => (
  <ProgressBarErrorBoundary>
    <ProgressBar {...props} />
  </ProgressBarErrorBoundary>
);
