import React, { useMemo, useState, useCallback, useEffect, useRef } from "react";
import {
  Box,
  Flex,
  Text,
  Tabs,
  IconButton,
  Badge,
  Button,
  Separator,
  Card,
} from "@radix-ui/themes";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  CopyIcon,
  DownloadIcon,
  DragHandleVerticalIcon,
} from "@radix-ui/react-icons";
import { useAppSelector } from "../../hooks";
import {
  selectMessages,
  selectChatId,
  getSelectedToolUse,
  selectModel,
  selectThreadMode,
  selectIntegration,
  selectSubchatUsageTotal,
  selectSubchatUsageByTool,
  selectThreadMaximumTokens,
  selectThreadCurrentMessageTokens,
} from "../../features/Chat/Thread/selectors";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";
import { ScrollArea } from "../ScrollArea";
import { MarkdownCodeBlock } from "../Markdown/CodeBlock";
import { isAssistantMessage } from "../../services/refact/types";
import {
  getChatPayloadDebug,
  type ChatPayloadDebugResponse,
} from "../../services/refact/chat";
import { selectLspPort, selectApiKey } from "../../features/Config/configSlice";
import styles from "./ContextPayloadSidebar.module.css";
import { TokenUsagePanel } from "./TokenUsagePanel";
import classNames from "classnames";

type ViewMode = "overview" | "messages" | "json" | "tokens" | "actual";

interface MessageBreakdown {
  role: string;
  count: number;
  totalTokens: number;
  avgTokens: number;
  hasToolCalls: number;
}

export type ContextPayloadSidebarVariant = "sidebar" | "page";

type ContextPayloadSidebarProps = {
  variant?: ContextPayloadSidebarVariant;
  onBackToChat?: () => void;
};

export const ContextPayloadSidebar: React.FC<ContextPayloadSidebarProps> = ({
  variant = "sidebar",
  onBackToChat,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>("overview");
  const [searchQuery] = useState("");
  const [expandedMessages, setExpandedMessages] = useState<Set<number>>(
    new Set()
  );
  const [actualPayload, setActualPayload] = useState<ChatPayloadDebugResponse | null>(null);
  const [isLoadingPayload, setIsLoadingPayload] = useState(false);
  const [payloadError, setPayloadError] = useState<string | null>(null);

  // Resize functionality
  const [panelWidth, setPanelWidth] = useState(() => {
    // Load from localStorage or use default (30% increase from 400px = 520px)
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem('contextPayloadPanelWidth');
      return saved ? parseInt(saved, 10) : 520;
    }
    return 520;
  });
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const chatId = useAppSelector(selectChatId);
  const messages = useAppSelector(selectMessages);
  const toolUse = useAppSelector(getSelectedToolUse);
  const model = useAppSelector(selectModel);
  const threadMode = useAppSelector(selectThreadMode);
  const integration = useAppSelector(selectIntegration);
  const lspPort = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey);
  const subchatUsage = useAppSelector(selectSubchatUsageTotal);
  const subchatUsageByTool = useAppSelector(selectSubchatUsageByTool);
  useAppSelector(selectThreadMaximumTokens);
  useAppSelector(selectThreadCurrentMessageTokens);

  // Format messages as they would be sent to the LLM
  const lspFormattedMessages = useMemo(() => {
    return formatMessagesForLsp(messages);
  }, [messages]);

  // Calculate comprehensive token estimates
  const tokenAnalysis = useMemo(() => {
    const jsonStr = JSON.stringify(lspFormattedMessages);
    const estimatedTotal = Math.ceil(jsonStr.length / 4);

    const byRole: Record<string, number> = {};
    const messageTokens: number[] = [];
    let totalToolCalls = 0;

    lspFormattedMessages.forEach((msg) => {
      const content =
        typeof msg.content === "string"
          ? msg.content
          : JSON.stringify(msg.content);
      const tokens = Math.ceil(content.length / 4);
      messageTokens.push(tokens);
      byRole[msg.role] = (byRole[msg.role] || 0) + tokens;

      if ("tool_calls" in msg && msg.tool_calls?.length) {
        totalToolCalls += msg.tool_calls.length;
      }
    });

    // Get actual usage from assistant messages
    const assistantMessages = messages.filter(isAssistantMessage);
    const actualUsage = assistantMessages.reduce(
      (acc, msg) => {
        if (msg.usage) {
          acc.prompt += msg.usage.prompt_tokens || 0;
          acc.completion += msg.usage.completion_tokens || 0;
          acc.total += msg.usage.total_tokens || 0;
        }
        return acc;
      },
      { prompt: 0, completion: 0, total: 0 }
    );

    return {
      estimated: estimatedTotal,
      byRole,
      messageTokens,
      totalToolCalls,
      actual: actualUsage,
      messageCount: lspFormattedMessages.length,
    };
  }, [lspFormattedMessages, messages]);

  // Calculate true Main Agent usage by subtracting Subchat usage from the total Actual usage
  // NOTE: some LLMs skip total_tokens in their response, so we derive total from prompt+completion
  const mainAgentUsage = useMemo(() => {
    const totalPrompt = tokenAnalysis.actual.prompt;
    const totalCompletion = tokenAnalysis.actual.completion;
    const subchatPrompt = subchatUsage?.prompt_tokens || 0;
    const subchatCompletion = subchatUsage?.completion_tokens || 0;

    const mainPrompt = Math.max(0, totalPrompt - subchatPrompt);
    const mainCompletion = Math.max(0, totalCompletion - subchatCompletion);

    return {
      prompt: mainPrompt,
      completion: mainCompletion,
      // Always derive total from p+c so we aren't blocked by missing total_tokens
      total: mainPrompt + mainCompletion,
      hasData: totalPrompt > 0 || totalCompletion > 0,
    };
  }, [tokenAnalysis.actual, subchatUsage]);

  // Message breakdown by role
  const messageBreakdown = useMemo(() => {
    const breakdown: Record<string, MessageBreakdown> = {};

    lspFormattedMessages.forEach((msg, idx) => {
      const role = msg.role;
      if (!breakdown[role]) {
        breakdown[role] = {
          role,
          count: 0,
          totalTokens: 0,
          avgTokens: 0,
          hasToolCalls: 0,
        };
      }

      breakdown[role].count++;
      const tokens = tokenAnalysis.messageTokens[idx] || 0;
      breakdown[role].totalTokens += tokens;

      if ("tool_calls" in msg && msg.tool_calls?.length) {
        breakdown[role].hasToolCalls++;
      }
    });

    Object.values(breakdown).forEach((b) => {
      b.avgTokens = Math.round(b.totalTokens / b.count);
    });

    return Object.values(breakdown);
  }, [lspFormattedMessages, tokenAnalysis.messageTokens]);

  // Filter messages based on search
  const filteredMessages = useMemo(() => {
    if (!searchQuery.trim()) return lspFormattedMessages;

    const query = searchQuery.toLowerCase();
    return lspFormattedMessages.filter((msg) => {
      const content =
        typeof msg.content === "string"
          ? msg.content
          : JSON.stringify(msg.content);
      return (
        msg.role.toLowerCase().includes(query) ||
        content.toLowerCase().includes(query)
      );
    });
  }, [lspFormattedMessages, searchQuery]);

  const toggleMessageExpansion = useCallback((index: number) => {
    setExpandedMessages((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  }, []);

  const copyToClipboard = useCallback(async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }, []);

  const fetchActualPayload = useCallback(async () => {
    if (messages.length === 0) return;

    setIsLoadingPayload(true);
    setPayloadError(null);

    try {
      const lspFormattedMessages = formatMessagesForLsp(messages);
      const payload = await getChatPayloadDebug({
        messages: lspFormattedMessages,
        model,
        port: lspPort,
        apiKey,
        onlyDeterministicMessages: false,
        chatId,
        checkpointsEnabled: true,
        integration: integration ? {
          path: integration.path || "",
        } : null,
        mode: threadMode,
        boost_reasoning: false,
        increase_max_tokens: false,
      });
      setActualPayload(payload);
    } catch (err) {
      setPayloadError(err instanceof Error ? err.message : "Failed to fetch payload");
      console.error("Failed to fetch actual payload:", err);
    } finally {
      setIsLoadingPayload(false);
    }
  }, [messages, model, lspPort, apiKey, chatId, integration, threadMode]);

  const exportPayload = useCallback(() => {
    const payload = {
      model,
      mode: threadMode,
      toolUse,
      timestamp: new Date().toISOString(),
      messageCount: lspFormattedMessages.length,
      tokenEstimate: tokenAnalysis.estimated,
      messages: lspFormattedMessages,
      actualPayload: actualPayload,
    };

    const blob = new Blob([JSON.stringify(payload, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `context-payload-${chatId}-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [
    model,
    threadMode,
    toolUse,
    lspFormattedMessages,
    tokenAnalysis.estimated,
    chatId,
    actualPayload,
  ]);

  // Handle resize
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (variant !== "sidebar") return;
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (panelRef.current) {
        const newWidth = e.clientX;
        const minWidth = 300;
        const maxWidth = 800;
        const clampedWidth = Math.max(minWidth, Math.min(maxWidth, newWidth));
        setPanelWidth(clampedWidth);
        try {
          if (typeof window !== 'undefined') {
            localStorage.setItem('contextPayloadPanelWidth', clampedWidth.toString());
          }
        } catch (err) {
          // Ignore localStorage errors
          console.warn('Failed to save panel width:', err);
        }
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing, variant]);

  if (variant === "sidebar" && isCollapsed) {
    return (
      <Box className={styles.collapsedContainer}>
        <IconButton
          variant="ghost"
          onClick={() => setIsCollapsed(false)}
          title="Expand Context Payload Panel"
        >
          <ChevronRightIcon />
        </IconButton>
      </Box>
    );
  }

  return (
    <Box
      ref={panelRef}
      className={classNames(styles.container, {
        [styles.pageContainer]: variant === "page",
      })}
      style={variant === "sidebar" ? { width: `${panelWidth}px` } : undefined}
    >
      {/* Header */}
      <Flex
        className={classNames(styles.header, {
          [styles.pageHeader]: variant === "page",
        })}
        style={{ paddingLeft: "0px" }}
        justify="between"
        align="center"
      >
        <Flex align="center" gap="2">
          {variant === "page" && onBackToChat && (
            <IconButton
              variant="ghost"
              size="2"
              onClick={onBackToChat}
              title="Back to chat"
            >
              <ChevronLeftIcon />
            </IconButton>
          )}
          <Text weight="bold" size="3">
            Context Payload
          </Text>
          <Badge color="blue" size="2">
            {tokenAnalysis.messageCount} msgs
          </Badge>
        </Flex>
        <Flex gap="1">
          <IconButton
            variant="ghost"
            size="2"
            onClick={() =>
              copyToClipboard(JSON.stringify(lspFormattedMessages, null, 2))
            }
            title="Copy JSON"
          >
            <CopyIcon />
          </IconButton>
          <IconButton
            variant="ghost"
            size="2"
            onClick={exportPayload}
            title="Export Payload"
          >
            <DownloadIcon />
          </IconButton>
          {variant === "sidebar" && (
            <IconButton
              variant="ghost"
              size="2"
              onClick={() => setIsCollapsed(true)}
              title="Collapse Panel"
            >
              <ChevronLeftIcon />
            </IconButton>
          )}
        </Flex>
      </Flex>



      {/* Tabs */}
      <Tabs.Root
        value={viewMode}
        onValueChange={(v) => setViewMode(v as ViewMode)}
        className={styles.tabsRoot}
      >
        <Tabs.List className={styles.tabsList}>
          <Tabs.Trigger value="overview">Overview</Tabs.Trigger>
          {/* <Tabs.Trigger value="messages">Messages</Tabs.Trigger> */}
          <Tabs.Trigger value="tokens">Tokens</Tabs.Trigger>
          {/* <Tabs.Trigger value="json">JSON</Tabs.Trigger>
          <Tabs.Trigger value="actual">Actual LLM Payload</Tabs.Trigger> */}
        </Tabs.List>

        <Box className={styles.tabContent}>
          {/* Overview Tab */}
          <Tabs.Content value="overview" className={styles.tabPane}>
            <ScrollArea style={{ height: "100%" }}>
              <Flex direction="column" gap="3" p="2">
                {/* Summary Stats */}
                <Card>
                  <Flex direction="column" gap="2">
                    <Text size="2" weight="bold">
                      Summary
                    </Text>
                    <Separator />
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Model:
                      </Text>
                      <Text size="1">{model}</Text>
                    </Flex>
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Mode:
                      </Text>
                      <Text size="1">{threadMode || "N/A"}</Text>
                    </Flex>
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Tool Use:
                      </Text>
                      <Text size="1">{toolUse}</Text>
                    </Flex>
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Total Messages:
                      </Text>
                      <Text size="1" weight="bold">
                        {tokenAnalysis.messageCount}
                      </Text>
                    </Flex>
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Tool Calls:
                      </Text>
                      <Text size="1" weight="bold">
                        {tokenAnalysis.totalToolCalls}
                      </Text>
                    </Flex>
                  </Flex>
                </Card>

                {/* Token Estimates */}
                <Card>
                  <Flex direction="column" gap="2">
                    <Text size="2" weight="bold">
                      Token Estimates
                    </Text>
                    <Separator />
                    <Flex justify="between">
                      <Text size="1" color="gray">
                        Estimated Total:
                      </Text>
                      <Text size="1" weight="bold">
                        ~{tokenAnalysis.estimated.toLocaleString()}
                      </Text>
                    </Flex>
                    {tokenAnalysis.actual.total > 0 && (
                      <>
                        <Separator />
                        <Text size="1" weight="bold" color="green">
                          Actual Usage (from responses)
                        </Text>
                        <Flex justify="between">
                          <Text size="1" color="gray">
                            Prompt:
                          </Text>
                          <Text size="1">
                            {tokenAnalysis.actual.prompt.toLocaleString()}
                          </Text>
                        </Flex>
                        <Flex justify="between">
                          <Text size="1" color="gray">
                            Completion:
                          </Text>
                          <Text size="1">
                            {tokenAnalysis.actual.completion.toLocaleString()}
                          </Text>
                        </Flex>
                        <Flex justify="between">
                          <Text size="1" color="gray">
                            Total:
                          </Text>
                          <Text size="1" weight="bold">
                            {tokenAnalysis.actual.total.toLocaleString()}
                          </Text>
                        </Flex>
                      </>
                    )}
                  </Flex>
                </Card>

                {/* Message Breakdown */}
                <Card>
                  <Flex direction="column" gap="2">
                    <Text size="2" weight="bold">
                      Message Breakdown
                    </Text>
                    <Separator />
                    {messageBreakdown.map((breakdown) => (
                      <Box key={breakdown.role}>
                        <Flex justify="between" align="center" mb="1">
                          <Badge
                            color={
                              breakdown.role === "user"
                                ? "green"
                                : breakdown.role === "assistant"
                                  ? "blue"
                                  : breakdown.role === "tool"
                                    ? "orange"
                                    : "gray"
                            }
                          >
                            {breakdown.role}
                          </Badge>
                          <Text size="1">{breakdown.count} messages</Text>
                        </Flex>
                        <Flex direction="column" gap="1" pl="4">
                          <Flex justify="between">
                            <Text size="1" color="gray">
                              Total tokens:
                            </Text>
                            <Text size="1">
                              {breakdown.totalTokens.toLocaleString()}
                            </Text>
                          </Flex>
                          <Flex justify="between">
                            <Text size="1" color="gray">
                              Avg per message:
                            </Text>
                            <Text size="1">{breakdown.avgTokens}</Text>
                          </Flex>
                          {breakdown.hasToolCalls > 0 && (
                            <Flex justify="between">
                              <Text size="1" color="gray">
                                With tool calls:
                              </Text>
                              <Text size="1">{breakdown.hasToolCalls}</Text>
                            </Flex>
                          )}
                        </Flex>
                        {messageBreakdown.indexOf(breakdown) <
                          messageBreakdown.length - 1 && <Separator my="2" />}
                      </Box>
                    ))}
                  </Flex>
                </Card>
              </Flex>
            </ScrollArea>
          </Tabs.Content>

          {/* Messages Tab */}
          <Tabs.Content value="messages" className={styles.tabPane}>
            <ScrollArea style={{ height: "100%" }}>
              <Flex direction="column" gap="2" p="2">
                {filteredMessages.length === 0 ? (
                  <Text size="2" color="gray" align="center">
                    No messages found
                  </Text>
                ) : (
                  filteredMessages.map((msg, idx) => {
                    const originalIndex = lspFormattedMessages.indexOf(msg);
                    const isExpanded = expandedMessages.has(originalIndex);
                    const content =
                      typeof msg.content === "string"
                        ? msg.content
                        : JSON.stringify(msg.content, null, 2);
                    const preview =
                      content.length > 150
                        ? content.slice(0, 150) + "..."
                        : content;

                    return (
                      <Card key={idx} className={styles.messageCard}>
                        <Flex direction="column" gap="2">
                          <Flex justify="between" align="center">
                            <Flex align="center" gap="2">
                              <Badge
                                color={
                                  msg.role === "user"
                                    ? "green"
                                    : msg.role === "assistant"
                                      ? "blue"
                                      : msg.role === "tool"
                                        ? "orange"
                                        : "gray"
                                }
                                size="2"
                              >
                                {msg.role}
                              </Badge>
                              {"tool_calls" in msg && msg.tool_calls?.length ? (
                                <Badge color="purple" size="1">
                                  🔧 {msg.tool_calls.length} tool(s)
                                </Badge>
                              ) : null}
                              <Text size="1" color="gray">
                                #{originalIndex + 1}
                              </Text>
                            </Flex>
                            <Button
                              size="1"
                              variant="ghost"
                              onClick={() =>
                                toggleMessageExpansion(originalIndex)
                              }
                            >
                              {isExpanded ? "Collapse" : "Expand"}
                            </Button>
                          </Flex>
                          <Box className={styles.messageContent}>
                            {isExpanded ? (
                              <MarkdownCodeBlock
                                useInlineStyles={true}
                                preOptions={{ noMargin: true }}
                              >
                                {content}
                              </MarkdownCodeBlock>
                            ) : (
                              <Text size="1" className={styles.previewText}>
                                {preview}
                              </Text>
                            )}
                          </Box>
                          {"tool_calls" in msg && msg.tool_calls ? (
                            <Box className={styles.toolCallsContainer}>
                              <Text size="1" weight="bold">
                                Tool Calls:
                              </Text>
                              {msg.tool_calls.map((tc, tcIdx) => (
                                <Box key={tcIdx} pl="2" mt="1">
                                  <Text size="1" color="purple">
                                    • {tc.function.name}
                                  </Text>
                                </Box>
                              ))}
                            </Box>
                          ) : null}
                        </Flex>
                      </Card>
                    );
                  })
                )}
              </Flex>
            </ScrollArea>
          </Tabs.Content>

          {/* Tokens Tab */}
          <Tabs.Content value="tokens" className={styles.tabPane}>
            <ScrollArea style={{ height: "100%" }}>
              <Flex direction="column" gap="3" p="2">
                <TokenUsagePanel
                  mainAgentUsage={mainAgentUsage}
                  tokenAnalysis={{ estimated: tokenAnalysis.estimated }}
                  subchatUsage={subchatUsage}
                  subchatUsageByTool={subchatUsageByTool}
                />
                <Card>
                  <Flex direction="column" gap="2">
                    <Text size="2" weight="bold">
                      Token Distribution
                    </Text>
                    <Separator />
                    {Object.entries(tokenAnalysis.byRole).map(
                      ([role, tokens]) => {
                        const percentage = Math.round(
                          (tokens / tokenAnalysis.estimated) * 100
                        );
                        return (
                          <Box key={role}>
                            <Flex justify="between" mb="1">
                              <Text size="1">{role}:</Text>
                              <Text size="1" weight="bold">
                                {tokens.toLocaleString()} ({percentage}%)
                              </Text>
                            </Flex>
                            <Box className={styles.progressBar}>
                              <Box
                                className={styles.progressFill}
                                style={{ width: `${percentage}%` }}
                              />
                            </Box>
                          </Box>
                        );
                      }
                    )}
                  </Flex>
                </Card>

                <Card>
                  <Flex direction="column" gap="2">
                    <Text size="2" weight="bold">
                      Per-Message Tokens
                    </Text>
                    <Separator />
                    {lspFormattedMessages.map((msg, idx) => (
                      <Flex
                        key={idx}
                        justify="between"
                        align="center"
                        py="1"
                        className={styles.tokenRow}
                      >
                        <Flex align="center" gap="2">
                          <Badge size="1">{msg.role}</Badge>
                          <Text size="1" color="gray">
                            #{idx + 1}
                          </Text>
                        </Flex>
                        <Text size="1">
                          ~
                          {tokenAnalysis.messageTokens[idx]?.toLocaleString() ||
                            0}
                        </Text>
                      </Flex>
                    ))}
                  </Flex>
                </Card>
              </Flex>
            </ScrollArea>
          </Tabs.Content>

          {/* JSON Tab */}
          <Tabs.Content value="json" className={styles.tabPane}>
            <ScrollArea style={{ height: "100%" }}>
              <Box className={styles.jsonContainer}>
                <MarkdownCodeBlock
                  useInlineStyles={true}
                  preOptions={{ noMargin: true }}
                >
                  {JSON.stringify(lspFormattedMessages, null, 2)}
                </MarkdownCodeBlock>
              </Box>
            </ScrollArea>
          </Tabs.Content>

          {/* Actual LLM Payload Tab */}
          <Tabs.Content value="actual" className={styles.tabPane}>
            <ScrollArea style={{ height: "100%" }}>
              <Flex direction="column" gap="3" p="2">
                <Card>
                  <Flex direction="column" gap="2">
                    <Flex justify="between" align="center">
                      <Text size="2" weight="bold">
                        Actual Payload Sent to LLM
                      </Text>
                      <Button
                        size="2"
                        onClick={fetchActualPayload}
                        disabled={isLoadingPayload || messages.length === 0}
                      >
                        {isLoadingPayload ? "Loading..." : "Fetch Payload"}
                      </Button>
                    </Flex>
                    <Separator />
                    {payloadError && (
                      <Box p="2" style={{ background: "var(--red-2)", borderRadius: "var(--radius-2)" }}>
                        <Text size="1" color="red">
                          Error: {payloadError}
                        </Text>
                      </Box>
                    )}
                    {actualPayload ? (
                      <Flex direction="column" gap="3">
                        <Card>
                          <Flex direction="column" gap="2">
                            <Text size="2" weight="bold">Metadata</Text>
                            <Separator />
                            <Flex justify="between">
                              <Text size="1" color="gray">Model:</Text>
                              <Text size="1">{actualPayload.model}</Text>
                            </Flex>
                            <Flex justify="between">
                              <Text size="1" color="gray">Model ID:</Text>
                              <Text size="1">{actualPayload.model_id}</Text>
                            </Flex>
                            <Flex justify="between">
                              <Text size="1" color="gray">Supports Tools:</Text>
                              <Text size="1">{actualPayload.supports_tools ? "Yes" : "No"}</Text>
                            </Flex>
                            <Flex justify="between">
                              <Text size="1" color="gray">Endpoint Style:</Text>
                              <Text size="1">{actualPayload.endpoint_style}</Text>
                            </Flex>
                          </Flex>
                        </Card>

                        {actualPayload.payload.tools && (
                          <Card>
                            <Flex direction="column" gap="2">
                              <Text size="2" weight="bold">
                                Tools ({Array.isArray(actualPayload.payload.tools) ? actualPayload.payload.tools.length : 0})
                              </Text>
                              <Separator />
                              <ScrollArea style={{ maxHeight: "200px" }}>
                                <MarkdownCodeBlock
                                  useInlineStyles={true}
                                  preOptions={{ noMargin: true }}
                                >
                                  {JSON.stringify(actualPayload.payload.tools, null, 2)}
                                </MarkdownCodeBlock>
                              </ScrollArea>
                            </Flex>
                          </Card>
                        )}

                        {actualPayload.payload.messages && (
                          <Card>
                            <Flex direction="column" gap="2">
                              <Text size="2" weight="bold">
                                Messages ({Array.isArray(actualPayload.payload.messages) ? actualPayload.payload.messages.length : 0})
                              </Text>
                              <Separator />
                              <ScrollArea style={{ maxHeight: "300px" }}>
                                <MarkdownCodeBlock
                                  useInlineStyles={true}
                                  preOptions={{ noMargin: true }}
                                >
                                  {JSON.stringify(actualPayload.payload.messages, null, 2)}
                                </MarkdownCodeBlock>
                              </ScrollArea>
                            </Flex>
                          </Card>
                        )}

                        {actualPayload.payload.prompt && (
                          <Card>
                            <Flex direction="column" gap="2">
                              <Text size="2" weight="bold">Prompt (Generic Model)</Text>
                              <Separator />
                              <ScrollArea style={{ maxHeight: "300px" }}>
                                <MarkdownCodeBlock
                                  useInlineStyles={true}
                                  preOptions={{ noMargin: true }}
                                >
                                  {actualPayload.payload.prompt}
                                </MarkdownCodeBlock>
                              </ScrollArea>
                            </Flex>
                          </Card>
                        )}

                        <Card>
                          <Flex direction="column" gap="2">
                            <Text size="2" weight="bold">Full Payload JSON</Text>
                            <Separator />
                            <ScrollArea style={{ maxHeight: "400px" }}>
                              <MarkdownCodeBlock
                                useInlineStyles={true}
                                preOptions={{ noMargin: true }}
                              >
                                {JSON.stringify(actualPayload.payload, null, 2)}
                              </MarkdownCodeBlock>
                            </ScrollArea>
                          </Flex>
                        </Card>
                      </Flex>
                    ) : (
                      <Box p="4">
                        <Text size="2" color="gray" align="center">
                          Click "Fetch Payload" to see the exact payload sent to the LLM
                        </Text>
                      </Box>
                    )}
                  </Flex>
                </Card>
              </Flex>
            </ScrollArea>
          </Tabs.Content>
        </Box>
      </Tabs.Root>

      {/* Resize Handle */}
      {variant === "sidebar" && (
        <Box
          className={styles.resizeHandle}
          onMouseDown={handleMouseDown}
          style={{ cursor: isResizing ? "col-resize" : "ew-resize" }}
          title="Drag to resize panel"
        >
          <DragHandleVerticalIcon />
        </Box>
      )}
    </Box>
  );
};

