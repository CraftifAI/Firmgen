import React, { useState, useEffect, useRef } from "react";
import {
  Box,
  Text,
  Button,
  Flex,
  Card,
  ScrollArea,
  IconButton,
} from "@radix-ui/themes";
import {
  TrashIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  CopyIcon,
} from "@radix-ui/react-icons";
import { useAppSelector } from "../../../hooks";
import {
  selectMessages,
  selectToolUse,
} from "../../../features/Chat/Thread/selectors";
import { ToolCall, ToolResult } from "../../../services/refact";
import styles from "./CommandLogWindow.module.css";

interface CommandEntry {
  id: string;
  timestamp: string;
  toolName: string;
  command: string;
  status: "running" | "success" | "error";
  exitCode?: number;
  duration?: number;
  stdout?: string;
  stderr?: string;
  expanded: boolean;
}

export const CommandLogWindow: React.FC = () => {
  const messages = useAppSelector(selectMessages);
  const [commandLogs, setCommandLogs] = useState<CommandEntry[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Extract tool calls and results from messages
  useEffect(() => {
    const logs: CommandEntry[] = [];
    let logIndex = 0;

    messages.forEach((message) => {
      if (message.role === "assistant" && message.tool_calls) {
        message.tool_calls.forEach((toolCall: ToolCall) => {
          if (toolCall.function?.name) {
            const toolName = toolCall.function.name;
            const args = toolCall.function.arguments || "{}";
            
            // Try to parse arguments to extract command
            let command = toolName;
            try {
              const parsedArgs = JSON.parse(args);
              if (parsedArgs.command) {
                command = parsedArgs.command;
              } else if (typeof parsedArgs === "string") {
                command = parsedArgs;
              } else {
                command = `${toolName}(${JSON.stringify(parsedArgs)})`;
              }
            } catch {
              command = `${toolName}(${args})`;
            }

            logs.push({
              id: toolCall.id || `log-${logIndex++}`,
              timestamp: new Date().toLocaleTimeString(),
              toolName,
              command,
              status: "running",
              expanded: false,
            });
          }
        });
      }

      if (message.role === "tool") {
        const toolResult = message.content as ToolResult;
        if (toolResult?.tool_call_id) {
          const logIndex = logs.findIndex(
            (log) => log.id === toolResult.tool_call_id
          );
          if (logIndex >= 0) {
            const content = toolResult.content;
            const isError = toolResult.tool_failed || false;
            
            // Try to extract stdout/stderr from content
            let stdout = "";
            let stderr = "";
            if (typeof content === "string") {
              // Try to parse structured output
              if (content.includes("STDOUT") || content.includes("STDERR")) {
                const stdoutMatch = content.match(/STDOUT\s*```\s*\n([\s\S]*?)```/);
                const stderrMatch = content.match(/STDERR\s*```\s*\n([\s\S]*?)```/);
                stdout = stdoutMatch ? stdoutMatch[1] : "";
                stderr = stderrMatch ? stderrMatch[1] : "";
              } else {
                stdout = content;
              }
            }

            // Try to extract exit code and duration
            const exitCodeMatch = content?.toString().match(/exit code (\d+)/);
            const durationMatch = content?.toString().match(/(\d+\.\d+)s/);
            
            logs[logIndex] = {
              ...logs[logIndex],
              status: isError ? "error" : "success",
              exitCode: exitCodeMatch ? parseInt(exitCodeMatch[1]) : undefined,
              duration: durationMatch ? parseFloat(durationMatch[1]) : undefined,
              stdout,
              stderr,
            };
          }
        }
      }
    });

    setCommandLogs(logs);
    
    // Auto-scroll to bottom
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  const handleClear = () => {
    setCommandLogs([]);
  };

  const handleToggleExpand = (id: string) => {
    setCommandLogs((prev) =>
      prev.map((log) =>
        log.id === id ? { ...log, expanded: !log.expanded } : log
      )
    );
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "success":
        return "var(--green-9)";
      case "error":
        return "var(--red-9)";
      default:
        return "var(--yellow-9)";
    }
  };

  return (
    <Card className={styles.container}>
      <Flex direction="column" height="100%">
        {/* <Flex align="center" justify="between" mb="2" pb="2" style={{ borderBottom: "1px solid var(--gray-6)" }}>
          <Text size="2" weight="bold">
            Command Log
          </Text>
          <Flex gap="1" align="center">
            <Text size="1" color="gray">
              {commandLogs.length} commands
            </Text>
            <IconButton
              size="1"
              variant="ghost"
              onClick={handleClear}
              title="Clear"
            >
              <TrashIcon />
            </IconButton>
          </Flex>
        </Flex> */}

        <ScrollArea className={styles.scrollArea} scrollbars="vertical">
          <Box className={styles.logsContainer}>
            {commandLogs.length === 0 ? (
              <Text size="2" color="gray" style={{ fontStyle: "italic" }}>
                No commands executed yet. Tool executions will appear here.
              </Text>
            ) : (
              commandLogs.map((log) => (
                <Box key={log.id} className={styles.logEntry}>
                  <Flex
                    align="center"
                    gap="2"
                    onClick={() => handleToggleExpand(log.id)}
                    style={{ cursor: "pointer" }}
                  >
                    {log.expanded ? (
                      <ChevronDownIcon width="12" height="12" />
                    ) : (
                      <ChevronRightIcon width="12" height="12" />
                    )}
                    <Box
                      style={{
                        width: "8px",
                        height: "8px",
                        borderRadius: "50%",
                        backgroundColor: getStatusColor(log.status),
                      }}
                    />
                    <Text size="1" color="gray" className={styles.timestamp}>
                      {log.timestamp}
                    </Text>
                    <Text size="2" weight="medium" className={styles.toolName}>
                      {log.toolName}
                    </Text>
                    {log.duration && (
                      <Text size="1" color="gray">
                        ({log.duration.toFixed(2)}s)
                      </Text>
                    )}
                  </Flex>

                  {log.expanded && (
                    <Box className={styles.logDetails}>
                      <Box className={styles.commandBox}>
                        <Flex align="center" justify="between" mb="1">
                          <Text size="1" weight="bold" color="gray">
                            Command:
                          </Text>
                          <IconButton
                            size="1"
                            variant="ghost"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCopy(log.command);
                            }}
                            title="Copy command"
                          >
                            <CopyIcon />
                          </IconButton>
                        </Flex>
                        <Text size="1" className={styles.commandText}>
                          {log.command}
                        </Text>
                      </Box>

                      {log.stdout && (
                        <Box className={styles.outputBox}>
                          <Text size="1" weight="bold" color="gray" mb="1">
                            STDOUT:
                          </Text>
                          <ScrollArea
                            className={styles.outputScroll}
                            scrollbars="vertical"
                          >
                            <Text size="1" className={styles.outputText}>
                              {log.stdout}
                            </Text>
                          </ScrollArea>
                        </Box>
                      )}

                      {log.stderr && (
                        <Box className={styles.outputBox}>
                          <Text size="1" weight="bold" color="red" mb="1">
                            STDERR:
                          </Text>
                          <ScrollArea
                            className={styles.outputScroll}
                            scrollbars="vertical"
                          >
                            <Text size="1" className={styles.outputText} style={{ color: "var(--red-9)" }}>
                              {log.stderr}
                            </Text>
                          </ScrollArea>
                        </Box>
                      )}

                      {log.exitCode !== undefined && (
                        <Text size="1" color="gray">
                          Exit code: {log.exitCode}
                        </Text>
                      )}
                    </Box>
                  )}
                </Box>
              ))
            )}
            <div ref={messagesEndRef} />
          </Box>
        </ScrollArea>
      </Flex>
    </Card>
  );
};