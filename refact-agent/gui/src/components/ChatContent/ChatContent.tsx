import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChatMessages,
  isAssistantMessage,
  isChatContextFileMessage,
  isDiffMessage,
  isToolMessage,
  isUserMessage,
  UserMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea, ScrollAreaWithAnchor } from "../ScrollArea";
import { Flex, Container, Button, Box, Text } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { PlainText } from "./PlainText";
import { useAppDispatch, useConfig, useDiffFileReload, useProgress } from "../../hooks";
import { useAppSelector } from "../../hooks";
import { fetchChatEsp32ProjectPath } from "../../services/refact/projectTree";
import {
  selectChatId,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThread,
} from "../../features/Chat/Thread/selectors";
import { takeWhile } from "../../utils";
import { GroupedDiffs } from "./DiffContent";
import { popBackTo } from "../../features/Pages/pagesSlice";
import { ChatLinks, UncommittedChangesWarning } from "../ChatLinks";
import { telemetryApi } from "../../services/refact/telemetry";
import { PlaceHolderText } from "./PlaceHolderText";
import { UsageCounter } from "../UsageCounter";
import {
  getConfirmationPauseStatus,
  getPauseReasonsWithPauseStatus,
} from "../../features/ToolConfirmation/confirmationSlice";
import { useUsageCounter } from "../UsageCounter/useUsageCounter.ts";
import { LogoAnimation } from "../LogoAnimation/LogoAnimation.tsx";
import { PirTopologyChatBlock } from "../../features/PirAgent";
import { usePirChatAnchor } from "../../features/PirAgent/hooks/usePirChatAnchor";
import { usePirCodegenReady } from "../../features/PirAgent/hooks/usePirCodegenReady";

export type ChatContentProps = {
  onRetry: (index: number, question: UserMessage["content"]) => void;
  onStopStreaming: () => void;
};

export const ChatContent: React.FC<ChatContentProps> = ({
  onStopStreaming,
  onRetry,
}) => {
  const dispatch = useAppDispatch();
  const pauseReasonsWithPause = useAppSelector(getPauseReasonsWithPauseStatus);
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const thread = useAppSelector(selectThread);
  const { shouldShow } = useUsageCounter();
  const isConfig = thread.mode === "CONFIGURE";
  const isWaiting = useAppSelector(selectIsWaiting);
  const chatId = useAppSelector(selectChatId);
  const agentIsWorking = useMemo(
    () =>
      (isWaiting || isStreaming) && !pauseReasonsWithPause.pause,
    [isWaiting, isStreaming, pauseReasonsWithPause.pause],
  );

  const progressApi = useProgress({
    chatId: chatId ?? undefined,
    pollingInterval: 1000,
  });
  const progressProjectPath = progressApi.progress?.esp32_project_path ?? null;
  const currentStage = progressApi.currentStage;

  // Fallback: derive project_path from chat messages when the progress session
  // hasn't captured it yet (e.g. agent wrote files directly without esp32_project).
  // Priority:
  //   1. esp32_* tool call inputs with an explicit project_path arg
  //   2. esp32_* tool RESULT messages with project_path in JSON output
  //   3. Any file-writing tool call whose path falls under a .../main/... directory
  //      (derive the ESP-IDF project root from that path)
  const toolCallProjectPath = useMemo(() => {
    if (progressProjectPath) return null;
    for (const msg of messages) {
      // Scan assistant message tool call inputs
      if (isAssistantMessage(msg) && msg.tool_calls) {
        for (const tc of msg.tool_calls) {
          const name = tc.function.name ?? "";
          try {
            const args = JSON.parse(tc.function.arguments) as Record<
              string,
              unknown
            >;
            // 1. esp32_* tool with explicit project_path input
            if (
              name.startsWith("esp32_") &&
              typeof args.project_path === "string" &&
              args.project_path.trim()
            ) {
              return args.project_path.trim();
            }
            // 3. File-writing tool whose path lands inside a main/ directory
            const filePath = (
              typeof args.path === "string"
                ? args.path
                : typeof args.filename === "string"
                  ? args.filename
                  : typeof args.file_path === "string"
                    ? args.file_path
                    : ""
            )
              .replace(/\\/g, "/")
              .trim();
            if (filePath.includes("/main/")) {
              const idx = filePath.lastIndexOf("/main/");
              const candidate = filePath.slice(0, idx);
              if (candidate) return candidate;
            }
          } catch { /* ignore malformed JSON */ }
        }
      }
      // 2. Scan tool result messages for project_path in JSON output
      if (isToolMessage(msg)) {
        const inner = msg.content;
        const contentStr =
          typeof inner.content === "string" ? inner.content : "";
        if (contentStr) {
          try {
            const parsed = JSON.parse(contentStr) as Record<string, unknown>;
            if (
              typeof parsed.project_path === "string" &&
              parsed.project_path.trim()
            ) {
              return parsed.project_path.trim();
            }
          } catch { /* not JSON, try regex */ }
          const m = /"project_path"\s*:\s*"([^"]+)"/.exec(contentStr);
          if (m?.[1]) return m[1].trim();
        }
      }
    }
    return null;
  }, [messages, progressProjectPath]);

  const config = useConfig();
  const lspPort = config.lspPort;

  // Fourth-level fallback: query /v1/esp32/chat-project-path once the agent
  // finishes, when no path was captured by progress-polling or tool-call scanning.
  // Covers cases where the agent wrote files without calling any esp32_* tool.
  const [autoDetectedPath, setAutoDetectedPath] = useState<string | null>(null);
  const hasInlinePath = Boolean(progressProjectPath ?? toolCallProjectPath);

  useEffect(() => {
    if (hasInlinePath || agentIsWorking || !chatId) return;
    let cancelled = false;
    let attempt = 0;
    const MAX_ATTEMPTS = 8;
    const tryDetect = () => {
      void fetchChatEsp32ProjectPath(chatId, lspPort).then((p) => {
        if (cancelled || hasInlinePath) return;
        if (p) {
          setAutoDetectedPath(p);
        } else if (attempt < MAX_ATTEMPTS) {
          attempt++;
          setTimeout(tryDetect, 2000);
        }
      });
    };
    tryDetect();
    return () => { cancelled = true; };
  }, [chatId, agentIsWorking, hasInlinePath, lspPort]);

  const effectiveProjectPath =
    progressProjectPath ?? toolCallProjectPath ?? autoDetectedPath;
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();
  const integrationMeta = useAppSelector(selectIntegration);
  const isWaitingForConfirmation = useAppSelector(getConfirmationPauseStatus);

  const onRetryWrapper = (index: number, question: UserMessage["content"]) => {
    onRetry(index, question);
  };

  const handleReturnToConfigurationClick = useCallback(() => {
    // console.log(`[DEBUG]: going back to configuration page`);
    // TBD: should it be allowed to run in the background?
    onStopStreaming();
    dispatch(
      popBackTo({
        name: "integrations page",
        projectPath: thread.integration?.project,
        integrationName: thread.integration?.name,
        integrationPath: thread.integration?.path,
        wasOpenedThroughChat: true,
      }),
    );
  }, [
    onStopStreaming,
    dispatch,
    thread.integration?.project,
    thread.integration?.name,
    thread.integration?.path,
  ]);

  const handleManualStopStreamingClick = useCallback(() => {
    onStopStreaming();
    void sendTelemetryEvent({
      scope: `stopStreaming`,
      success: true,
      error_message: "",
    });
  }, [onStopStreaming, sendTelemetryEvent]);

  const shouldConfigButtonBeVisible = useMemo(() => {
    return isConfig && !integrationMeta?.path?.includes("project_summary");
  }, [isConfig, integrationMeta?.path]);

  const lastAgentTurnId = useMemo(() => {
    if (agentIsWorking) return null;
    let lastIndex = -1;
    messages.forEach((m, i) => {
      if (isAssistantMessage(m)) lastIndex = i;
    });
    return lastIndex >= 0 ? `assistant-${lastIndex}` : null;
  }, [messages, agentIsWorking]);

  const pirChatContext = useMemo(() => {
    return messages
      .filter((m) => m.role === "user" || m.role === "assistant")
      .slice(-6)
      .map((m) => {
        if (typeof m.content === "string") {
          return `${m.role.toUpperCase()}: ${m.content.slice(0, 400)}`;
        }
        return null;
      })
      .filter(Boolean)
      .join("\n");
  }, [messages]);

  const { ready: pirCodegenReady, checking: pirCodegenChecking } = usePirCodegenReady({
    chatId: chatId ?? null,
    projectPath: effectiveProjectPath,
    agentTurnId: lastAgentTurnId,
    agentIsWorking,
  });

  const { anchorTurnId, effectiveProjectPath: pirAnchorProjectPath, showBlock: showPirBlock } = usePirChatAnchor({
    chatId: chatId ?? null,
    projectPath: effectiveProjectPath,
    codegenReady: pirCodegenReady,
    agentTurnId: lastAgentTurnId,
    agentIsWorking,
  });

  const pirTopologyBlock = useMemo(() => {
    if (!showPirBlock || !chatId || !pirAnchorProjectPath || !anchorTurnId) return null;
    return (
      <PirTopologyChatBlock
        key={`pir-${chatId}-${anchorTurnId}`}
        chatId={chatId}
        projectPath={pirAnchorProjectPath}
        currentStage={currentStage}
        isAgentStreaming={isStreaming}
        agentTurnId={anchorTurnId}
        chatContext={pirChatContext}
        codegenReady={pirCodegenReady}
        codegenChecking={pirCodegenChecking}
      />
    );
  }, [showPirBlock, chatId, pirAnchorProjectPath, anchorTurnId, currentStage, isStreaming, pirChatContext, pirCodegenReady, pirCodegenChecking]);

  const renderedMessages = useMemo(
    () => renderMessages(messages, onRetryWrapper, pirTopologyBlock, anchorTurnId),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [messages, pirTopologyBlock, anchorTurnId],
  );

  // Dedicated hook for handling file reloads
  useDiffFileReload();

  return (
    <ScrollAreaWithAnchor.ScrollArea
      style={{ flexGrow: 1, height: "auto", position: "relative" }}
      scrollbars="vertical"
      type={isWaiting || isStreaming ? "auto" : "hover"}
      fullHeight
      className={messages.length === 0 ? styles.scrollAreaEmpty : undefined}
    >
      <Flex
        direction="column"
        className={styles.content}
        data-element="ChatContent"
        p="2"
        gap="1"
        style={messages.length === 0 ? { minHeight: "100%" } : undefined}
      >
        {messages.length === 0 && (
          <Flex
            flexGrow="1"
            direction="column"
            align="center"
            justify="center"
            className={styles.placeholderCenter}
          >
            <PlaceHolderText />
          </Flex>
        )}
        {renderedMessages}
        {agentIsWorking && (
          <Flex
            className={styles.agentWorkingBelowBlob}
            aria-live="polite"
            aria-busy="true"
            aria-label="processing, please wait"
          >
            <Text as="div" size="1" className={styles.workingLine}>
              <span className={styles.workingVerb}>processing</span>
              <span className={styles.animatedDots} aria-hidden="true">
                <span className={styles.animatedDot} />
                <span className={styles.animatedDot} />
                <span className={styles.animatedDot} />
              </span>
            </Text>
          </Flex>
        )}
        <Container>
          <UncommittedChangesWarning />
        </Container>
        {shouldShow && <UsageCounter />}
        {/* <Container pt="4" pb="8">
          {!isWaitingForConfirmation && (
            <LogoAnimation
              size="8"
              isStreaming={isStreaming}
              isWaiting={isWaiting}
            />
          )}
        </Container> */}
      </Flex>

      <Box
        style={{
          position: "absolute",
          bottom: 0,
          maxWidth: "100%", // TODO: make space for the down button
        }}
      >
        <ScrollArea scrollbars="horizontal">
          <Flex align="start" gap="3" pb="2">
            {/* {(isWaiting || isStreaming) && !pauseReasonsWithPause.pause && (
              <Button
                // ml="auto"
                size="2"
                variant="solid"
                title="Stop streaming"
                onClick={handleManualStopStreamingClick}
              >
                Stop
              </Button>
            )} */}
            {shouldConfigButtonBeVisible && (
              <Button
                // ml="auto"
                color="gray"
                title="Return to configuration page"
                onClick={handleReturnToConfigurationClick}
              >
                Return
              </Button>
            )}

            <ChatLinks />
          </Flex>
        </ScrollArea>
      </Box>
    </ScrollAreaWithAnchor.ScrollArea>
  );
};

ChatContent.displayName = "ChatContent";

function renderMessages(
  messages: ChatMessages,
  onRetry: (index: number, question: UserMessage["content"]) => void,
  pirTopologyBlock: React.ReactNode | null = null,
  pirAnchorTurnId: string | null = null,
  memo: React.ReactNode[] = [],
  index = 0,
) {
  if (messages.length === 0) return memo;
  const [head, ...tail] = messages;
  if (head.role === "tool") {
    return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, memo, index + 1);
  }

  if (head.role === "plain_text") {
    const key = "plain-text-" + index;
    const nextMemo = [...memo, <PlainText key={key}>{head.content}</PlainText>];
    return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, nextMemo, index + 1);
  }

  if (head.role === "assistant") {
    const key = "assistant-input-" + index;
    const turnId = `assistant-${index}`;
    const isLast = !tail.some(isAssistantMessage);
    const pirBlock =
      pirTopologyBlock && pirAnchorTurnId && turnId === pirAnchorTurnId
        ? pirTopologyBlock
        : null;
    const nextMemo = [
      ...memo,
      <AssistantInput
        key={key}
        message={head.content}
        reasoningContent={head.reasoning_content}
        toolCalls={head.tool_calls}
        isLast={isLast}
      />,
      ...(pirBlock ? [pirBlock] : []),
    ];

    return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, nextMemo, index + 1);
  }

  if (head.role === "user") {
    const key = "user-input-" + index;
    const isLastUserMessage = !tail.some(isUserMessage);
    const nextMemo = [
      ...memo,
      isLastUserMessage && (
        <ScrollAreaWithAnchor.ScrollAnchor
          key={`${key}-anchor`}
          behavior="smooth"
          block="start"
        // my="-2"
        />
      ),
      <UserInput onRetry={onRetry} key={key} messageIndex={index}>
        {head.content}
      </UserInput>,
    ];
    return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, nextMemo, index + 1);
  }

  if (isChatContextFileMessage(head)) {
    const key = "context-file-" + index;
    const nextMemo = [...memo, <ContextFiles key={key} files={head.content} />];
    return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, nextMemo, index + 1);
  }

  if (isDiffMessage(head)) {
    const restInTail = takeWhile(tail, (message) => {
      return isDiffMessage(message) || isToolMessage(message);
    });

    const nextTail = tail.slice(restInTail.length);
    const diffMessages = [head, ...restInTail.filter(isDiffMessage)];
    const key = "diffs-" + index;

    const nextMemo = [...memo, <GroupedDiffs key={key} diffs={diffMessages} />];

    return renderMessages(nextTail, onRetry, pirTopologyBlock, pirAnchorTurnId, nextMemo, index + diffMessages.length);
  }

  return renderMessages(tail, onRetry, pirTopologyBlock, pirAnchorTurnId, memo, index + 1);
}
