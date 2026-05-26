import { FileTextIcon, Pencil2Icon } from "@radix-ui/react-icons";
import { Badge, Button, Container, Flex, IconButton, Text } from "@radix-ui/themes";
import React, { useCallback, useMemo, useState } from "react";
import { selectMessages } from "../../features/Chat";
import { CheckpointButton } from "../../features/Checkpoints";
import { useAppSelector } from "../../hooks";
import {
  isUserMessage,
  ProcessedUserMessageContentWithImages,
  UserMessageContentWithImage,
  type UserMessage,
} from "../../services/refact";
import { takeWhile } from "../../utils";
import { RetryForm } from "../ChatForm";
import { DialogImage } from "../DialogImage";
import { Markdown } from "../Markdown";
import styles from "./ChatContent.module.css";
import { Reveal } from "../Reveal";

// ── File Attachment Badge Extractors ──────────────────────────────────────────

const FILES_SUFFIX_REGEX = / \(files: ([^)]+)\)$/;

function parseQueryAndFiles(content: string): {
  query: string;
  fileNames: string[];
} {
  const match = content.match(FILES_SUFFIX_REGEX);
  if (!match) return { query: content, fileNames: [] };
  const query = content.slice(0, match.index).trim();
  const fileNames = match[1].split(",").map((f) => f.trim()).filter(Boolean);
  return { query, fileNames };
}

function renderFileBadges(fileNames: string[]): JSX.Element | null {
  if (fileNames.length === 0) return null;
  return (
    <Flex key="attached-files" gap="1" wrap="wrap" mt="2" align="center">
      {fileNames.map((name) => (
        <Badge
          key={name}
          size="1"
          variant="soft"
          color="blue"
          style={{ gap: "4px", alignItems: "center" }}
        >
          <FileTextIcon width="12" height="12" />
          {name}
        </Badge>
      ))}
    </Flex>
  );
}

// ──────────────────────────────────────────────────────────────────────────────

export type UserInputProps = {
  children: UserMessage["content"];
  messageIndex: number;
  // maybe add images argument ?
  onRetry: (index: number, question: UserMessage["content"]) => void;
  /** Renders directly under the user message bubble (e.g. agent working indicator). */
  belowBubble?: React.ReactNode;
  // disableRetry?: boolean;
};

export const UserInput: React.FC<UserInputProps> = ({
  messageIndex,
  children,
  onRetry,
  belowBubble,
}) => {
  const messages = useAppSelector(selectMessages);

  const [showTextArea, setShowTextArea] = useState(false);
  const [isEditButtonVisible, setIsEditButtonVisible] = useState(false);

  const handleSubmit = useCallback(
    (value: UserMessage["content"]) => {
      onRetry(messageIndex, value);
      setShowTextArea(false);
    },
    [messageIndex, onRetry],
  );

  const handleShowTextArea = useCallback(
    (value: boolean) => {
      setShowTextArea(value);
      if (isEditButtonVisible) {
        setIsEditButtonVisible(false);
      }
    },
    [isEditButtonVisible],
  );

  // const lines = children.split("\n"); // won't work if it's an array
  const elements = process(children);
  const isString = typeof children === "string";
  const linesLength = isString ? children.split("\n").length : Infinity;

  const checkpointsFromMessage = useMemo(() => {
    const maybeUserMessage = messages[messageIndex];
    if (!isUserMessage(maybeUserMessage)) return null;
    return maybeUserMessage.checkpoints;
  }, [messageIndex, messages]);

  const isCompressed = useMemo(() => {
    if (typeof children !== "string") return false;
    return children.startsWith("🗜️ ");
  }, [children]);

  return (
    <Container position="relative" pt="1">
      {isCompressed ? (
        <Flex direction="column" align="start" gap="0" width="100%">
          <Reveal defaultOpen={false}>
            <Flex
              direction="row"
              my="1"
              className={`${styles.userInput} ${styles.userMessageBubble}`}
            >
              {elements}
            </Flex>
          </Reveal>
          {belowBubble}
        </Flex>
      ) : showTextArea ? (
        <RetryForm
          onSubmit={handleSubmit}
          // TODO
          // value={children}
          value={children}
          onClose={() => handleShowTextArea(false)}
        />
      ) : (
        <Flex direction="column" align="start" gap="0" width="100%">
          <Flex
            direction="row"
            // checking for the length of the lines to determine the position of the edit button
            gap={linesLength <= 2 ? "2" : "1"}
            // TODO: what is it's a really long sentence or word with out new lines?
            align={linesLength <= 2 ? "center" : "end"}
            my="1"
            onMouseEnter={() => setIsEditButtonVisible(true)}
            onMouseLeave={() => setIsEditButtonVisible(false)}
          >
            <Button
              // ref={ref}
              variant="soft"
              size="4"
              className={styles.userInput}
              // TODO: should this work?
              // onClick={() => handleShowTextArea(true)}
              asChild
            >
              <div className={styles.userMessageBubble}>{elements}</div>
            </Button>
            <Flex
              direction={linesLength <= 3 ? "row" : "column"}
              gap="1"
              style={{
                opacity: isEditButtonVisible ? 1 : 0,
                visibility: isEditButtonVisible ? "visible" : "hidden",
                transition: "opacity 0.15s, visibility 0.15s",
              }}
            >
              {checkpointsFromMessage && checkpointsFromMessage.length > 0 && (
                <CheckpointButton
                  checkpoints={checkpointsFromMessage}
                  messageIndex={messageIndex}
                />
              )}
              <IconButton
                title="Edit message"
                variant="soft"
                size={"2"}
                onClick={() => handleShowTextArea(true)}
              >
                <Pencil2Icon width={15} height={15} />
              </IconButton>
            </Flex>
          </Flex>
          {belowBubble}
        </Flex>
      )}
    </Container>
  );
};

function process(items: UserInputProps["children"]) {
  if (typeof items !== "string") {
    return processUserInputArray(items);
  }
  const { query, fileNames } = parseQueryAndFiles(items);
  const elements = processLines(query.split("\n"));
  const badges = renderFileBadges(fileNames);
  if (badges) elements.push(badges);
  return elements;
}

function processLines(
  lines: string[],
  processedLinesMemo: JSX.Element[] = [],
): JSX.Element[] {
  if (lines.length === 0) return processedLinesMemo;

  const [head, ...tail] = lines;
  const nextBackTicksIndex = tail.findIndex((l) => l.startsWith("```"));
  const key = `line-${processedLinesMemo.length + 1}`;

  if (!head.startsWith("```") || nextBackTicksIndex === -1) {
    const processedLines = processedLinesMemo.concat(
      <Text
        size="2"
        as="div"
        key={key}
        wrap="balance"
        className={styles.break_word}
      >
        {head}
      </Text>,
    );
    return processLines(tail, processedLines);
  }

  const endIndex = nextBackTicksIndex + 1;

  const code = [head].concat(tail.slice(0, endIndex)).join("\n");
  const processedLines = processedLinesMemo.concat(
    <Markdown key={key}>{code}</Markdown>,
  );

  const next = tail.slice(endIndex);
  return processLines(next, processedLines);
}

function isUserContentImage(
  item: UserMessageContentWithImage | ProcessedUserMessageContentWithImages,
) {
  return (
    ("m_type" in item && item.m_type.startsWith("image/")) ||
    ("type" in item && item.type === "image_url")
  );
}

function processUserInputArray(
  items: (
    | UserMessageContentWithImage
    | ProcessedUserMessageContentWithImages
  )[],
  memo: JSX.Element[] = [],
) {
  if (items.length === 0) return memo;
  const [head, ...tail] = items;

  if ("type" in head && head.type === "text") {
    const { query, fileNames } = parseQueryAndFiles(head.text);
    const processedLines = processLines(query.split("\n"));
    const badges = renderFileBadges(fileNames);
    if (badges) processedLines.push(badges);
    return processUserInputArray(tail, memo.concat(processedLines));
  }

  if ("m_type" in head && head.m_type === "text") {
    const { query, fileNames } = parseQueryAndFiles(head.m_content);
    const processedLines = processLines(query.split("\n"));
    const badges = renderFileBadges(fileNames);
    if (badges) processedLines.push(badges);
    return processUserInputArray(tail, memo.concat(processedLines));
  }

  const isImage = isUserContentImage(head);

  if (!isImage) return processUserInputArray(tail, memo);

  const imagesInTail = takeWhile(tail, isUserContentImage);
  const nextTail = tail.slice(imagesInTail.length);
  const images = [head, ...imagesInTail];
  const elem = (
    <Flex key={`user-image-images-${memo.length}`} gap="2" wrap="wrap" my="2">
      {images.map((image, index) => {
        if ("type" in image && image.type === "image_url") {
          const key = `user-input${memo.length}-${image.type}-${index}`;
          const content = image.image_url.url;
          return <DialogImage src={content} key={key} />;
        }
        if ("m_type" in image && image.m_type.startsWith("image/")) {
          const key = `user-input${memo.length}-${image.m_type}-${index}`;
          const content = `data:${image.m_type};base64,${image.m_content}`;
          return <DialogImage src={content} key={key} />;
        }
        return null;
      })}
    </Flex>
  );

  return processUserInputArray(nextTail, memo.concat(elem));
}
