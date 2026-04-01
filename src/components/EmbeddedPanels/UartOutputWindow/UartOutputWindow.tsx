import React, { useState, useEffect, useRef, useMemo } from "react";
import {
  Box,
  Text,
  Flex,
  Card,
  ScrollArea,
  IconButton,
} from "@radix-ui/themes";
import {
  TrashIcon,
  PlayIcon,
  PauseIcon,
  DownloadIcon,
} from "@radix-ui/react-icons";
import { useAppSelector } from "../../../hooks";
import { selectMessages } from "../../../features/Chat/Thread/selectors";
import {
  isToolMessage,
  isAssistantMessage,
  isChatContextFileMessage,
  type ToolMessage,
  type ToolCall,
  type ToolResult,
  type ChatContextFile,
  isMultiModalToolResult,
  isSingleModelToolResult,
} from "../../../services/refact/types";
import styles from "./UartOutputWindow.module.css";

interface UartMessage {
  timestamp: string;
  content: string;
  type: "info" | "error" | "warning";
}

export const UartOutputWindow: React.FC = () => {
  const messages = useAppSelector(selectMessages);
  const [isPaused, setIsPaused] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [clearedMessageCount, setClearedMessageCount] = useState(0); // Track how many messages to skip
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Extract UART output from chat messages
  const uartMessages = useMemo(() => {
    const extractedMessages: UartMessage[] = [];

    // Look for ESP32 device monitor tool calls
    messages.forEach((message) => {
      if (isAssistantMessage(message) && message.tool_calls) {
        message.tool_calls.forEach((toolCall: ToolCall) => {
          // Check for ESP32 device monitor operation
          if (toolCall.function?.name === "esp32_device" && toolCall.function?.arguments && toolCall.id) {
            try {
              const args = JSON.parse(toolCall.function.arguments);
              if (args.operation === "monitor") {
                // Find the corresponding tool result message
                const toolResultMessage = messages.find((msg) => {
                  if (isToolMessage(msg)) {
                    return msg.content.tool_call_id === toolCall.id;
                  }
                  return false;
                });

                if (toolResultMessage && isToolMessage(toolResultMessage)) {
                  const toolResult: ToolResult = toolResultMessage.content;
                  let content = "";

                  if (isSingleModelToolResult(toolResult)) {
                    content = typeof toolResult.content === "string" ? toolResult.content : "";
                  } else if (isMultiModalToolResult(toolResult)) {
                    content = toolResult.content
                      .filter((item) => item.m_type === "text")
                      .map((item) => item.m_content)
                      .join("");
                  }

                  if (content) {
                    // ESP32 output format: "STDOUT\n```\n{actual_output}\n```"
                    // Extract content between ``` markers
                    const codeBlockMatch = content.match(/STDOUT\s*\n```\s*\n([\s\S]*?)\n```/);
                    if (codeBlockMatch && codeBlockMatch[1]) {
                      const uartOutput = codeBlockMatch[1];
                      const lines = uartOutput.split("\n");
                      lines.forEach((line) => {
                        extractedMessages.push({
                          timestamp: new Date().toLocaleTimeString(),
                          content: line,
                          type: toolResult.tool_failed ? "error" : "info",
                        });
                      });
                    } else {
                      // Fallback: try to extract from JSON data field
                      try {
                        const jsonMatch = content.match(/```json\s*\n([\s\S]*?)\n```/);
                        if (jsonMatch) {
                          const jsonData = JSON.parse(jsonMatch[1]);
                          if (jsonData.data && jsonData.data.details) {
                            const details = jsonData.data.details;
                            const stdoutMatch = details.match(/STDOUT\s*\n```\s*\n([\s\S]*?)\n```/);
                            if (stdoutMatch && stdoutMatch[1]) {
                              const uartOutput = stdoutMatch[1];
                              const lines = uartOutput.split("\n");
                              lines.forEach((line: string) => {
                                extractedMessages.push({
                                  timestamp: new Date().toLocaleTimeString(),
                                  content: line,
                                  type: toolResult.tool_failed ? "error" : "info",
                                });
                              });
                            }
                          }
                        }
                      } catch {
                        // If parsing fails, try to extract raw content (skip metadata lines)
                        const lines = content.split("\n");
                        lines.forEach((line) => {
                          const trimmed = line.trim();
                          if (trimmed && !trimmed.includes("STDOUT") && !trimmed.includes("```") && !trimmed.startsWith("STDERR")) {
                            extractedMessages.push({
                              timestamp: new Date().toLocaleTimeString(),
                              content: line,
                              type: toolResult.tool_failed ? "error" : "info",
                            });
                          }
                        });
                      }
                    }
                  }
                }
              }
            } catch (e) {
              // Failed to parse arguments, skip
            }
          }

          // Look for cat tool calls that read UART capture files (C2000 compatibility)
          // UART capture files follow the pattern: uart_capture_*.txt
          if (toolCall.function?.name === "cat" && toolCall.function?.arguments && toolCall.id) {
            try {
              const args = JSON.parse(toolCall.function.arguments);
              // The cat tool uses "paths" as a comma-separated string
              let paths: string[] = [];
              if (args.paths) {
                if (typeof args.paths === "string") {
                  // Split comma-separated paths
                  paths = args.paths.split(",").map((p: string) => p.trim());
                } else if (Array.isArray(args.paths)) {
                  paths = args.paths;
                }
              } else if (args.path) {
                // Fallback to singular "path" if it exists
                paths = typeof args.path === "string" ? [args.path] : args.path;
              }

              // Check if any of the paths match UART capture file pattern
              const isUartCaptureFile = paths.some((path: string) =>
                typeof path === "string" && path.includes("uart_capture_") && path.endsWith(".txt")
              );

              if (isUartCaptureFile) {
                // Find the corresponding tool result message
                const toolResultMessage = messages.find((msg) => {
                  if (isToolMessage(msg)) {
                    return msg.content.tool_call_id === toolCall.id;
                  }
                  return false;
                });

                if (toolResultMessage && isToolMessage(toolResultMessage)) {
                  const toolResult: ToolResult = toolResultMessage.content;
                  let content = "";

                  if (isSingleModelToolResult(toolResult)) {
                    content = typeof toolResult.content === "string" ? toolResult.content : "";
                  } else if (isMultiModalToolResult(toolResult)) {
                    // Handle multimodal content (extract text parts)
                    content = toolResult.content
                      .filter((item) => item.m_type === "text")
                      .map((item) => item.m_content)
                      .join("");
                  }

                  if (content) {
                    // Filter out cat tool status messages and extract only UART data
                    const lines = content.split("\n");
                    const uartDataLines: string[] = [];
                    let inUartData = false;

                    for (const line of lines) {
                      const trimmed = line.trim();
                      // Skip empty lines at the start
                      if (!trimmed && !inUartData) continue;
                      // Skip cat tool status messages
                      if (trimmed.startsWith("Paths found:") ||
                        trimmed.startsWith("Symbols not found") ||
                        trimmed.startsWith("Problems:") ||
                        trimmed === "") {
                        // Skip empty lines between status and data
                        if (trimmed === "" && !inUartData) continue;
                        continue;
                      }
                      // Once we hit actual content, include everything
                      if (trimmed) {
                        inUartData = true;
                      }
                      // Include the line if we're in UART data section or if it's non-empty
                      if (inUartData || trimmed) {
                        uartDataLines.push(line);
                      }
                    }

                    // Add the UART data lines
                    uartDataLines.forEach((line) => {
                      extractedMessages.push({
                        timestamp: new Date().toLocaleTimeString(),
                        content: line,
                        type: toolResult.tool_failed ? "error" : "info",
                      });
                    });
                  }
                }
              }
            } catch (e) {
              // Failed to parse arguments, skip
            }
          }
        });
      }
    });

    // Also check for context_file messages that contain UART capture files
    // (cat tool returns text files as context_file messages)
    messages.forEach((message) => {
      if (isChatContextFileMessage(message)) {
        message.content.forEach((file: ChatContextFile) => {
          // Check if this is a UART capture file
          if (file.file_name && file.file_name.includes("uart_capture_") && file.file_name.endsWith(".txt")) {
            // Extract file content
            if (file.file_content) {
              const lines = file.file_content.split("\n");
              lines.forEach((line) => {
                // Include all lines (empty lines might be intentional for UART output)
                extractedMessages.push({
                  timestamp: new Date().toLocaleTimeString(),
                  content: line,
                  type: "info",
                });
              });
            }
          }
        });
      }
    });

    return extractedMessages;
  }, [messages]);

  // Filter out cleared messages
  const displayedMessages = useMemo(() => {
    return uartMessages.slice(clearedMessageCount);
  }, [uartMessages, clearedMessageCount]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (autoScroll && messagesEndRef.current && !isPaused) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [displayedMessages, autoScroll, isPaused]);

  const handleClear = () => {
    // Clear by setting the count to the current message count
    // This effectively hides all currently displayed messages
    setClearedMessageCount(uartMessages.length);
  };

  const handlePause = () => {
    setIsPaused(!isPaused);
  };

  const handleDownload = () => {
    // Download all messages (not just displayed ones)
    const content = uartMessages
      .map((msg) => `[${msg.timestamp}] ${msg.content}`)
      .join("\n");
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `uart-output-${new Date().toISOString()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const getMessageColor = (type: string) => {
    switch (type) {
      case "error":
        return "var(--red-9)";
      case "warning":
        return "var(--yellow-9)";
      default:
        return "var(--gray-12)";
    }
  };

  // Check if there are any UART-related tool calls to show connection status
  const hasUartToolCalls = useMemo(() => {
    return messages.some((message) => {
      if (isAssistantMessage(message) && message.tool_calls) {
        return message.tool_calls.some((toolCall: ToolCall) => {
          // Check for ESP32 device monitor
          if (toolCall.function?.name === "esp32_device" && toolCall.function?.arguments) {
            try {
              const args = JSON.parse(toolCall.function.arguments);
              return args.operation === "monitor";
            } catch {
              return false;
            }
          }
          // Check for C2000 UART capture tool (backward compatibility)
          if (toolCall.function?.name === "c2000_uart_capture") {
            return true;
          }
          // Check for cat commands reading UART files (C2000 compatibility)
          if (toolCall.function?.name === "cat" && toolCall.function?.arguments) {
            try {
              const args = JSON.parse(toolCall.function.arguments);
              // The cat tool uses "paths" as a comma-separated string
              let paths: string[] = [];
              if (args.paths) {
                if (typeof args.paths === "string") {
                  paths = args.paths.split(",").map((p: string) => p.trim());
                } else if (Array.isArray(args.paths)) {
                  paths = args.paths;
                }
              } else if (args.path) {
                paths = typeof args.path === "string" ? [args.path] : args.path;
              }
              return paths.some((path: string) =>
                typeof path === "string" && path.includes("uart_capture_") && path.endsWith(".txt")
              );
            } catch {
              return false;
            }
          }
          return false;
        });
      }
      return false;
    });
  }, [messages]);

  return (
    <Card className={styles.container}>
      <Flex direction="column" height="100%">
        {/* <Flex align="center" justify="between" mb="2" pb="2" style={{ borderBottom: "1px solid var(--gray-6)" }}>
          <Text size="2" weight="bold">
            ESP32 UART Output
          </Text>
          <Flex gap="1" align="center">
            <Box
              style={{
                width: "8px",
                height: "8px",
                borderRadius: "50%",
                backgroundColor: hasUartToolCalls ? "var(--green-9)" : "var(--gray-6)",
              }}
            />
            <Text size="1" color="gray">
              {hasUartToolCalls ? "Active" : "Waiting"}
            </Text>
            <IconButton
              size="1"
              variant="ghost"
              onClick={handlePause}
              title={isPaused ? "Resume" : "Pause"}
            >
              {isPaused ? <PlayIcon /> : <PauseIcon />}
            </IconButton>
            <IconButton
              size="1"
              variant="ghost"
              onClick={handleClear}
              title="Clear"
            >
              <TrashIcon />
            </IconButton>
            <IconButton
              size="1"
              variant="ghost"
              onClick={handleDownload}
              title="Download"
            >
              <DownloadIcon />
            </IconButton>
          </Flex>
        </Flex> */}

        <ScrollArea
          ref={scrollAreaRef}
          className={styles.scrollArea}
          scrollbars="vertical"
        >
          <Box className={styles.messagesContainer}>
            {displayedMessages.length === 0 ? (
              <Text size="2" color="gray" style={{ fontStyle: "italic" }}>
                {hasUartToolCalls
                  ? "Waiting for UART output..."
                  : "No UART output yet. Use esp32_device tool with 'monitor' operation to start streaming."}
              </Text>
            ) : (
              displayedMessages.map((msg, index) => (
                <Box key={index} className={styles.message}>
                  <Text size="1" color="gray" className={styles.timestamp}>
                    {msg.timestamp}
                  </Text>
                  <Text
                    size="2"
                    style={{ color: getMessageColor(msg.type) }}
                    className={styles.content}
                  >
                    {msg.content}
                  </Text>
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

