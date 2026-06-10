import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Box, Flex, Spinner, Text } from "@radix-ui/themes";
import { ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import * as Collapsible from "@radix-ui/react-collapsible";
import { FiFile, FiFolder } from "react-icons/fi";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  selectChatId,
  selectThread,
} from "../../features/Chat/Thread";
import { ScrollArea } from "../ScrollArea";
import {
  fetchChatEsp32ProjectPath,
  fetchProjectTree,
  openProjectTreeFile,
  resolveActiveEsp32ProjectPath,
  type ProjectTreeNode,
} from "../../services/refact/projectTree";
import { setInformation } from "../../features/Errors/informationSlice";
import sidebarStyles from "../ChatHistorySidebar/ChatHistorySidebar.module.css";
import styles from "./ProjectFileTree.module.css";

const POLL_MS_LIVE = 1000;
const DEBOUNCE_MS = 150;
const LS_EXPANDED_KEY = "projectFileTreeSectionExpanded";

type TreeRowProps = {
  node: ProjectTreeNode;
  depth: number;
  onOpenFile: (filePath: string) => void;
};

const TreeRow: React.FC<TreeRowProps> = React.memo(({ node, depth, onOpenFile }) => {
  const isDir = node.type === "dir";
  const [expanded, setExpanded] = useState(depth < 2);

  const handleRowClick = () => {
    if (isDir) {
      setExpanded((v) => !v);
      return;
    }
    onOpenFile(node.path);
  };

  const handleChevronClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setExpanded((v) => !v);
  };

  return (
    <Box>
      <Flex
        align="center"
        className={`${styles.treeRow} ${isDir ? styles.treeRowDir : ""}`}
        style={{ paddingLeft: `calc(var(--space-2) + ${depth * 12}px)` }}
        onClick={handleRowClick}
        title={node.path}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleRowClick();
          }
        }}
      >
        {isDir ? (
          <span
            className={styles.treeChevronBtn}
            onClick={handleChevronClick}
            aria-label={expanded ? "Collapse folder" : "Expand folder"}
          >
            {expanded ? (
              <ChevronDownIcon width="12" height="12" />
            ) : (
              <ChevronRightIcon width="12" height="12" />
            )}
          </span>
        ) : (
          <span className={styles.treeChevronSpacer} aria-hidden />
        )}
        {isDir ? (
          <FiFolder size={14} className={styles.treeIcon} aria-hidden />
        ) : (
          <FiFile size={14} className={styles.treeIcon} aria-hidden />
        )}
        <span className={styles.treeLabel}>{node.name.replace(/\/$/, "")}</span>
      </Flex>
      {isDir && expanded && node.children && node.children.length > 0 && (
        <Box>
          {node.children.map((child) => (
            <TreeRow
              key={child.path}
              node={child}
              depth={depth + 1}
              onOpenFile={onOpenFile}
            />
          ))}
        </Box>
      )}
    </Box>
  );
});
TreeRow.displayName = "TreeRow";

type ProjectFileTreeProps = {
  progressProjectPath?: string | null;
};

export const ProjectFileTree: React.FC<ProjectFileTreeProps> = ({
  progressProjectPath = null,
}) => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const thread = useAppSelector(selectThread);
  const threadProjectPath = thread.esp32_projects_path?.trim() ?? "";
  const lspPort = useAppSelector((s) => s.config.lspPort);

  const [sectionExpanded, setSectionExpanded] = useState(() => {
    if (typeof localStorage === "undefined") return true;
    const saved = localStorage.getItem(LS_EXPANDED_KEY);
    return saved !== "false";
  });
  const [tree, setTree] = useState<ProjectTreeNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [apiProjectPath, setApiProjectPath] = useState<string | null>(null);
  const lastFetchedRootRef = useRef<string | null>(null);
  const refreshTimerRef = useRef<number | null>(null);
  const refreshInFlightRef = useRef(false);
  const pendingRefreshRef = useRef<
    { force?: boolean; background?: boolean } | undefined
  >(undefined);

  const projectRoot = useMemo(
    () =>
      resolveActiveEsp32ProjectPath({
        chatId,
        messages: thread.messages,
        progressProjectPath,
        apiProjectPath,
        threadProjectPath,
      }),
    [chatId, thread.messages, progressProjectPath, apiProjectPath, threadProjectPath],
  );

  const projectRootRef = useRef(projectRoot);
  const lspPortRef = useRef(lspPort);
  projectRootRef.current = projectRoot;
  lspPortRef.current = lspPort;

  const hasStableProjectPath = Boolean(
    threadProjectPath || progressProjectPath?.trim(),
  );

  const refreshProjectPath = useCallback(async () => {
    if (hasStableProjectPath || !chatId) {
      if (!hasStableProjectPath) setApiProjectPath(null);
      return;
    }
    const path = await fetchChatEsp32ProjectPath(chatId, lspPort);
    setApiProjectPath(path);
  }, [chatId, lspPort, hasStableProjectPath]);

  const refreshTree = useCallback(
    async (options?: { force?: boolean; background?: boolean }) => {
      const force = options?.force ?? false;
      const background = options?.background ?? false;
      const root = projectRootRef.current;

      if (!root) {
        setTree([]);
        setError(null);
        setLoading(false);
        lastFetchedRootRef.current = null;
        return;
      }
      if (refreshInFlightRef.current) {
        pendingRefreshRef.current = options ?? { force: true, background: true };
        return;
      }
      if (!force && lastFetchedRootRef.current === root) {
        return;
      }

      refreshInFlightRef.current = true;
      if (!background) {
        setLoading(true);
      }
      setError(null);
      try {
        const res = await fetchProjectTree(root, lspPortRef.current, 8, { force });
        setTree(res.tree);
        lastFetchedRootRef.current = root;
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        if (!background) {
          setTree([]);
        }
      } finally {
        refreshInFlightRef.current = false;
        if (!background) {
          setLoading(false);
        }
        const pending = pendingRefreshRef.current;
        pendingRefreshRef.current = undefined;
        if (pending) {
          void refreshTree(pending);
        }
      }
    },
    [],
  );

  const scheduleRefresh = useCallback(
    (options?: { force?: boolean; background?: boolean }) => {
      if (!sectionExpanded) return;
      if (refreshTimerRef.current) {
        window.clearTimeout(refreshTimerRef.current);
      }
      refreshTimerRef.current = window.setTimeout(() => {
        refreshTimerRef.current = null;
        if (!hasStableProjectPath) {
          void refreshProjectPath();
        }
        void refreshTree(options);
      }, DEBOUNCE_MS);
    },
    [refreshProjectPath, refreshTree, sectionExpanded, hasStableProjectPath],
  );

  useEffect(() => {
    if (!sectionExpanded) return;
    lastFetchedRootRef.current = null;
    scheduleRefresh({ force: true, background: false });
  }, [projectRoot, sectionExpanded, scheduleRefresh]);

  useEffect(() => {
    if (!sectionExpanded || !projectRoot) return;

    const tick = () => {
      if (document.visibilityState !== "visible") return;
      void refreshTree({ force: true, background: true });
    };

    tick();
    const id = window.setInterval(tick, POLL_MS_LIVE);
    return () => window.clearInterval(id);
  }, [sectionExpanded, projectRoot, refreshTree]);

  useEffect(() => {
    if (!sectionExpanded || !projectRoot) return;
    const onWake = () => {
      if (document.visibilityState !== "visible") return;
      void refreshTree({ force: true, background: true });
    };
    window.addEventListener("focus", onWake);
    document.addEventListener("visibilitychange", onWake);
    return () => {
      window.removeEventListener("focus", onWake);
      document.removeEventListener("visibilitychange", onWake);
    };
  }, [sectionExpanded, projectRoot, refreshTree]);

  useEffect(
    () => () => {
      if (refreshTimerRef.current) window.clearTimeout(refreshTimerRef.current);
    },
    [],
  );

  const handleOpenFile = useCallback(
    (filePath: string) => {
      if (!projectRoot) return;
      void (async () => {
        try {
          await openProjectTreeFile(projectRoot, filePath, lspPort);
        } catch (err) {
          dispatch(
            setInformation(
              `Could not open file: ${
                err instanceof Error ? err.message : String(err)
              }`,
            ),
          );
        }
      })();
    },
    [dispatch, projectRoot, lspPort],
  );

  const handleSectionOpenChange = (open: boolean) => {
    setSectionExpanded(open);
    try {
      localStorage.setItem(LS_EXPANDED_KEY, open ? "true" : "false");
    } catch {
      // ignore
    }
    if (open) scheduleRefresh({ force: true, background: false });
  };

  return (
    <Collapsible.Root
      className={styles.projectFileTreeRoot}
      open={sectionExpanded}
      onOpenChange={handleSectionOpenChange}
    >
      <Collapsible.Trigger asChild>
        <button type="button" className={sidebarStyles.projectsSectionTrigger}>
          <ChevronDownIcon
            width="14"
            height="14"
            className={`${sidebarStyles.projectsChevron} ${
              sectionExpanded
                ? sidebarStyles.projectsChevronOpen
                : sidebarStyles.projectsChevronClosed
            }`}
          />
          <Text as="span" weight="medium">
            Project files
          </Text>
        </button>
      </Collapsible.Trigger>
      <Collapsible.Content className={styles.projectFileTreeContent}>
        {!projectRoot ? (
          <Text size="1" className={styles.emptyState}>
            No ESP-IDF project yet. Ask the agent to create one (e.g. esp32_project
            create).
          </Text>
        ) : loading && tree.length === 0 ? (
          <FlexLoading />
        ) : error ? (
          <Text size="1" className={styles.errorState}>
            {error}
          </Text>
        ) : tree.length === 0 ? (
          <Text size="1" className={styles.emptyState}>
            Project folder is empty.
          </Text>
        ) : (
          <>
            <Text size="1" className={styles.pathHint} title={projectRoot}>
              {projectRoot.split(/[/\\]/).pop() ?? projectRoot}
            </Text>
            <ScrollArea scrollbars="vertical" className={styles.projectFileTreeScroll}>
              <Box className={styles.treeList}>
                {tree.map((node) => (
                  <TreeRow
                    key={node.path}
                    node={node}
                    depth={0}
                    onOpenFile={handleOpenFile}
                  />
                ))}
              </Box>
            </ScrollArea>
          </>
        )}
      </Collapsible.Content>
    </Collapsible.Root>
  );
};

const FlexLoading: React.FC = () => (
  <Box className={styles.loadingRow}>
    <Spinner size="1" />
    <Text size="1">Loading files…</Text>
  </Box>
);
