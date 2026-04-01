import React, { useState } from "react";
import { Flex, Box, Text, IconButton } from "@radix-ui/themes";
import { ChevronLeftIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { useSelector } from "react-redux";

import { WorkflowPanel } from "../WorkflowPanel";
// import { UartOutputWindow } from "./UartOutputWindow";
// import { CommandLogWindow } from "./CommandLogWindow";
// import { VecDbStatusPanel } from "./VecDbStatusPanel";
import { useWorkflow } from "../../hooks/useWorkflow";
import { selectChatId } from "../../features/Chat/Thread/selectors";
import styles from "./EmbeddedPanels.module.css";

class ErrorBoundary extends React.Component<
  { children: React.ReactNode; name: string },
  { hasError: boolean; error?: Error }
> {
  constructor(props: { children: React.ReactNode; name: string }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error(`Error in ${this.props.name}:`, error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <Box p="2">
          <Text size="1" color="red">
            Error loading {this.props.name}
          </Text>
        </Box>
      );
    }

    return this.props.children;
  }
}

const WorkflowPanelContainer: React.FC = () => {
  const chatId = useSelector(selectChatId);
  const { snapshot, pause, resume, cancel, skipTask, retryTask, isLoading } =
    useWorkflow({ chatId });

  return (
    <WorkflowPanel
      snapshot={snapshot}
      onPause={pause}
      onResume={resume}
      onCancel={cancel}
      onSkipTask={skipTask}
      onRetryTask={retryTask}
      isLoading={isLoading}
    />
  );
};

export const EmbeddedPanels: React.FC = () => {
  const [isCollapsed, setIsCollapsed] = useState(false);

  React.useEffect(() => {
    console.log("EmbeddedPanels component mounted!");
  }, []);

  if (isCollapsed) {
    return (
      <Box className={styles.panelsContainerCollapsed}>
        <IconButton
          variant="ghost"
          onClick={() => setIsCollapsed(false)}
          title="Expand Workflow Tasks Panel"
          className={styles.collapseButton}
        >
          <ChevronRightIcon />
        </IconButton>
      </Box>
    );
  }

  return (
    <Box className={styles.panelsContainer}>
      <Flex
        direction="column"
        gap="2"
        height="100%"
        className={styles.panelsContent}
      >
        <Flex
          className={styles.workflowHeader}
          align="center"
          gap="2"
          onClick={(e) => e.stopPropagation()}
        >
          <IconButton
            variant="ghost"
            size="1"
            onClick={() => setIsCollapsed(true)}
            title="Collapse Workflow Tasks Panel"
            className={styles.collapseButton}
          >
            <ChevronLeftIcon />
          </IconButton>
          <Text size="2" weight="bold">
            Workflow Tasks
          </Text>
        </Flex>
        <Box className={styles.primaryPanel}>
          <ErrorBoundary name="WorkflowPanel">
            <WorkflowPanelContainer />
          </ErrorBoundary>
        </Box>
        {/* <Box className={styles.panel}>
          <ErrorBoundary name="UartOutputWindow">
            <UartOutputWindow />
          </ErrorBoundary>
        </Box>
        <Box className={styles.panel}>
          <ErrorBoundary name="CommandLogWindow">
            <CommandLogWindow />
          </ErrorBoundary>
        </Box>
        <Box className={styles.panel}>
          <ErrorBoundary name="VecDbStatusPanel">
            <VecDbStatusPanel />
          </ErrorBoundary>
        </Box> */}
      </Flex>
    </Box>
  );
};
