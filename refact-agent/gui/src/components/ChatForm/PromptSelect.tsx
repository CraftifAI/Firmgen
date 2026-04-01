import React, { useCallback, useMemo } from "react";
import { Flex, Skeleton, Text, Box } from "@radix-ui/themes";
import { Select } from "../Select";
import type { SystemPrompts } from "../../services/refact";
import {
  useAppDispatch,
  useAppSelector,
  useGetPromptsQuery,
  useGetCapsQuery,
} from "../../hooks";
import { getSelectedSystemPrompt } from "../../features/Chat/Thread/selectors";
import { setSystemPrompt, setToolUse, setChatMode } from "../../features/Chat/Thread/actions";
import { chatModeToLspMode } from "../../features/Chat/Thread/types";

export const PromptSelect: React.FC = () => {
  const dispatch = useAppDispatch();
  const promptsRequest = useGetPromptsQuery();
  const selectedSystemPrompt = useAppSelector(getSelectedSystemPrompt);
  const onSetSelectedSystemPrompt = useCallback(
    (prompt: SystemPrompts) => dispatch(setSystemPrompt(prompt)),
    [dispatch],
  );

  const handleChange = useCallback(
    (key: string) => {
      if (!promptsRequest.data) return;
      if (!(key in promptsRequest.data)) return;
      const promptValue = promptsRequest.data[key];
      const prompt = { [key]: promptValue };
      onSetSelectedSystemPrompt(prompt);
      
      // Map system prompt keys to tool_use and chat mode
      let toolUse: "quick" | "explore" | "agent" | undefined;
      if (key === "exploration_tools") {
        toolUse = "explore";
      } else if (key === "agentic_tools") {
        toolUse = "agent";
      } else if (key === "default") {
        toolUse = "quick";
      }
      
      // Update tool_use and chat mode if mapping exists
      if (toolUse) {
        dispatch(setToolUse(toolUse));
        const mode = chatModeToLspMode({ toolUse });
        dispatch(setChatMode(mode));
      }
    },
    [onSetSelectedSystemPrompt, promptsRequest.data, dispatch],
  );

  const caps = useGetCapsQuery();

  const default_system_prompt = useMemo(() => {
    if (
      caps.data?.code_chat_default_system_prompt &&
      caps.data.code_chat_default_system_prompt !== ""
    ) {
      return caps.data.code_chat_default_system_prompt;
    }
    return "default";
  }, [caps.data?.code_chat_default_system_prompt]);

  const val = useMemo(
    () => Object.keys(selectedSystemPrompt)[0] ?? default_system_prompt,
    [selectedSystemPrompt, default_system_prompt],
  );

  const options = useMemo(() => {
    return Object.entries(promptsRequest.data ?? {}).map(([key, value]) => {
      // Add mode indicators to the title
      let title = value.description || value.text;
      if (key === "exploration_tools") {
        title = `[Mode 2] ${title || "Exploration Tools"}`;
      } else if (key === "agentic_tools") {
        title = `[Mode 3] ${title || "Agentic Tools"}`;
      } else if (key === "default") {
        title = `[Mode 1] ${title || "Default"}`;
      }
      
      return {
        value: key,
        title: title,
      };
    });
  }, [promptsRequest.data]);

  const isLoading = useMemo(
    () =>
      promptsRequest.isLoading || promptsRequest.isFetching || caps.isLoading,
    [promptsRequest.isLoading, promptsRequest.isFetching, caps.isLoading],
  );

  if (options.length <= 1) return null;

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
    >
      {/* <Text size="2" wrap="nowrap">
        System Prompt:
      </Text>
      <Skeleton loading={isLoading}>
        <Box flexGrow="1" flexShrink="0">
          <Select
            name="system prompt"
            disabled={promptsRequest.isLoading}
            onChange={handleChange}
            value={val}
            options={options}
          />
        </Box>
      </Skeleton> */}
    </Flex>
  );
};
