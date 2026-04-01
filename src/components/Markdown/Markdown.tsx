import React, { Key, useMemo, useCallback } from "react";
import ReactMarkdown, { Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import classNames from "classnames";
// import "./highlightjs.css";
import styles from "./Markdown.module.css";
import {
  MarkdownCodeBlock,
  type MarkdownCodeBlockProps,
  type MarkdownControls,
} from "./CodeBlock";
import {
  Text,
  Heading,
  Blockquote,
  Em,
  Kbd,
  Link,
  Quote,
  Strong,
  Flex,
  Table,
} from "@radix-ui/themes";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import remarkGfm from "remark-gfm";
import "katex/dist/katex.min.css";
import { useLinksFromLsp } from "../../hooks";
import { setInputValue } from "../ChatForm/actions";

import { ChatLinkButton } from "../ChatLinks";
import { extractLinkFromPuzzle } from "../../utils/extractLinkFromPuzzle";

export type MarkdownProps = Pick<
  React.ComponentProps<typeof ReactMarkdown>,
  "children" | "allowedElements" | "unwrapDisallowed"
> &
  Pick<
    MarkdownCodeBlockProps,
    | "startingLineNumber"
    | "showLineNumbers"
    | "useInlineStyles"
    | "style"
    | "color"
  > & {
    canHaveInteractiveElements?: boolean;
    wrap?: boolean;
  } & Partial<MarkdownControls>;

const PuzzleLink: React.FC<{
  children: string;
}> = ({ children }) => {
  const { handleLinkAction } = useLinksFromLsp();
  const link = extractLinkFromPuzzle(children);

  if (!link) return children;

  return (
    <Flex direction="column" align="start" gap="2" mt="2">
      <ChatLinkButton link={link} onClick={handleLinkAction} />
    </Flex>
  );
};

const MaybeInteractiveElement: React.FC<{
  key?: Key | null;
  children?: React.ReactNode;
}> = ({ children }) => {
  const processed = React.Children.map(children, (child, index) => {
    if (typeof child === "string" && child.startsWith("🧩")) {
      const key = `puzzle-link-${index}`;
      return <PuzzleLink key={key}>{child}</PuzzleLink>;
    }
    return child;
  });

  return (
    <Text className={styles.maybe_pin} my="2">
      {processed}
    </Text>
  );
};

const _Markdown: React.FC<MarkdownProps> = ({
  children,
  allowedElements,
  unwrapDisallowed,
  canHaveInteractiveElements,
  color,
  ...rest
}) => {
  const handleOptionClick = useCallback((optionText: string) => {
    const placeholderIdx = optionText.indexOf("___");
    const isTemplate = placeholderIdx !== -1;

    const textToInsert = isTemplate
      ? optionText.substring(0, placeholderIdx)
      : optionText;
    const suffix = isTemplate ? "" : ", ";

    const textarea = document.querySelector<HTMLTextAreaElement>(
      '[data-testid="chat-form-textarea"]',
    );

    if (textarea) {
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const currentValue = textarea.value;

      let prefix = currentValue.substring(0, start);
      const rest = currentValue.substring(end);

      if (prefix.length > 0 && !prefix.endsWith(" ")) {
        prefix += " ";
      }

      const insertion = textToInsert + suffix;
      const newValue = prefix + insertion + rest;

      window.postMessage(
        setInputValue({ value: newValue, send_immediately: false }),
        "*",
      );

      setTimeout(() => {
        textarea.focus();
        const cursorPosition = prefix.length + insertion.length;
        textarea.setSelectionRange(cursorPosition, cursorPosition);
      }, 0);
    } else {
      window.postMessage(
        setInputValue({
          value: textToInsert + suffix,
          send_immediately: false,
        }),
        "*",
      );
    }
  }, []);

  const components: Partial<Components> = useMemo(() => {
    return {
      ol(props) {
        return (
          <ol {...props} className={classNames(styles.list, props.className)} />
        );
      },
      ul(props) {
        return (
          <ul {...props} className={classNames(styles.list, props.className)} />
        );
      },
      li(props) {
        const childrenArray = React.Children.toArray(props.children);
        if (canHaveInteractiveElements && childrenArray.length === 1) {
          const firstChild = childrenArray[0];

          const tryRenderButton = (rawText: string) => {
            const text = rawText.trim();
            if (!text.startsWith("[") || !text.endsWith("]")) return null;
            const optionText = text.slice(1, -1).trim();
            if (!optionText) return null;

            const isTemplate = optionText.includes("___");
            const displayText = isTemplate
              ? optionText.replace("___", "\u2026")
              : optionText;

            return (
              <div style={{ marginBottom: "8px" }}>
                <button
                  type="button"
                  className={classNames(
                    styles.option_button,
                    isTemplate && styles.option_button_template,
                  )}
                  onClick={(e) => {
                    e.preventDefault();
                    handleOptionClick(optionText);
                  }}
                >
                  {displayText}
                </button>
              </div>
            );
          };

          // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access
          if (React.isValidElement(firstChild) && firstChild.props.node?.tagName === "p") {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-argument
            const pChildrenArray = React.Children.toArray(firstChild.props.children);
            if (pChildrenArray.length === 1 && typeof pChildrenArray[0] === "string") {
              const btn = tryRenderButton(pChildrenArray[0]);
              if (btn) return btn;
            }
          } else if (typeof firstChild === "string") {
            const btn = tryRenderButton(firstChild);
            if (btn) return btn;
          }
        }
        return <li {...props} />;
      },
      code({ style: _style, color: _color, ...props }) {
        return <MarkdownCodeBlock color={color} {...props} {...rest} />;
      },
      p({ color: _color, ref: _ref, node: _node, ...props }) {
        if (canHaveInteractiveElements) {
          return <MaybeInteractiveElement {...props} />;
        }
        return <Text as="p" {...props} />;
      },
      h1({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="4" size="4" as="h1" {...props} />;
      },
      h2({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h2" {...props} />;
      },
      h3({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h3" {...props} />;
      },
      h4({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h4" {...props} />;
      },
      h5({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h5" {...props} />;
      },
      h6({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Heading my="3" size="3" as="h6" {...props} />;
      },
      blockquote({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Blockquote {...props} />;
      },
      em({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Em {...props} />;
      },
      kbd({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Kbd {...props} />;
      },
      a({ color: _color, ref: _ref, node: _node, ...props }) {
        const shouldTargetBeBlank =
          props.href &&
          (props.href.startsWith("http") || props.href.startsWith("https"));
        return (
          <Link
            {...props}
            target={shouldTargetBeBlank ? "_blank" : undefined}
          />
        );
      },
      q({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Quote {...props} />;
      },
      strong({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Strong {...props} />;
      },
      b({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Text {...props} weight="bold" />;
      },
      i({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Em {...props} />;
      },
      table({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Root my="2" variant="surface" {...props} />;
      },
      tbody({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Body {...props} />;
      },
      thead({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Header {...props} />;
      },
      tr({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.Row {...props} />;
      },
      th({ color: _color, ref: _ref, node: _node, ...props }) {
        return <Table.ColumnHeaderCell {...props} />;
      },
      td({ color: _color, ref: _ref, node: _node, width: _width, ...props }) {
        return <Table.Cell {...props} />;
      },
    };
  }, [rest, canHaveInteractiveElements, color, handleOptionClick]);
  return (
    <ReactMarkdown
      className={styles.markdown}
      remarkPlugins={[remarkBreaks, remarkMath, remarkGfm]}
      rehypePlugins={[rehypeKatex]}
      allowedElements={allowedElements}
      unwrapDisallowed={unwrapDisallowed}
      components={components}
    >
      {children}
    </ReactMarkdown>
  );
};

export const Markdown = React.memo(_Markdown);
