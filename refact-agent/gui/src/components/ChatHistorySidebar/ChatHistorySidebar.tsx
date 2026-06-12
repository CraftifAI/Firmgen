import React, { useState, useCallback, useEffect, useRef, useMemo } from "react";
import {
  Box,
  Flex,
  Text,
  IconButton,
  Button,
  TextField,
  DropdownMenu,
  Dialog,
} from "@radix-ui/themes";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ChevronDownIcon,
  ChatBubbleIcon,
  MagnifyingGlassIcon,
  Cross1Icon,
  DotsVerticalIcon,
  DragHandleVerticalIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
import * as Collapsible from "@radix-ui/react-collapsible";
import { FiFolder, FiFolderPlus } from "react-icons/fi";
import { v4 as uuidv4 } from "uuid";
import {
  useAppSelector,
  useAppDispatch,
  useEventsBusForIDE,
  useGetUser,
  useCraftifAuth,
} from "../../hooks";
import {
  getHistory,
  deleteChatById,
  setChatProjectById,
  type ChatHistoryItem,
} from "../../features/History/historySlice";
import { chatToHtml } from "../../utils/chatExportHtml";
import {
  restoreChat,
  selectChatId,
  selectThread,
  setChatProject,
} from "../../features/Chat/Thread";
import { popBackTo, push } from "../../features/Pages/pagesSlice";
import { ScrollArea } from "../ScrollArea";
import styles from "./ChatHistorySidebar.module.css";
import { newChatAction } from "../../events";
import { clearPauseReasonsAndHandleToolsStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { Dropdown, type DropdownNavigationOptions } from "../Toolbar/Dropdown";
import {
  createEsp32ProjectWorkspace,
  defaultEspWorkspaceParentCandidates,
  setDefaultEspWorkspaceParent,
  slugifyEspWorkspaceFolderName,
} from "../../services/refact/esp32ProjectWorkspace";
import {
  addProject,
  removeProject,
  setActiveProjectId,
  setProjectsSectionExpanded,
  type WorkspaceProject,
} from "../../features/WorkspaceProjects/workspaceProjectsSlice";
import { openProjectSourcesInFileManager } from "../../services/refact/projectSources";
import { setInformation } from "../../features/Errors/informationSlice";
import { ProjectFileTree } from "../ProjectFileTree";
import { TopologyMinimapSection } from "../../features/PirAgent/components/TopologyMinimapSection";
import type { PipelineStage } from "../../hooks/useWorkflowStatus";

type CraftifDesktopBridge = {
  browseFolder?: () => Promise<string | null>;
};

function getCraftifDesktopBridge(): CraftifDesktopBridge | undefined {
  if (typeof window === "undefined") return undefined;
  return (window as Window & { craftifai?: CraftifDesktopBridge }).craftifai;
}

const PANEL_WIDTH_KEY = "chatHistorySidebarWidth";
const DEFAULT_WIDTH = 260;
const MAX_VISIBLE_PROJECTS = 5;

export const ChatHistorySidebar: React.FC<{
  progressProjectPath?: string | null;
  currentStage?: PipelineStage;
}> = ({ progressProjectPath = null, currentStage = "PLANNING" }) => {
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const { displayName: craftifDisplayName } = useCraftifAuth();
  const profileName =
    craftifDisplayName?.trim() ||
    user.data?.fuser_id ||
    "User";
  const profileInitial = profileName.trim().charAt(0).toUpperCase() || "U";
  const { openSettings, openHotKeys } = useEventsBusForIDE();
  const history = useAppSelector((app) => app.history, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const currentChatId = useAppSelector(selectChatId);
  const currentThread = useAppSelector(selectThread);
  const lspPort = useAppSelector((s) => s.config.lspPort);
  const projects = useAppSelector((s) => s.workspaceProjects.projects);
  const activeProjectId = useAppSelector((s) => s.workspaceProjects.activeProjectId);
  const projectsSectionExpanded = useAppSelector(
    (s) => s.workspaceProjects.projectsSectionExpanded,
  );

  const [isCollapsed, setIsCollapsed] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedProjectIds, setExpandedProjectIds] = useState<Record<string, boolean>>({});

  const [panelWidth, setPanelWidth] = useState(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(PANEL_WIDTH_KEY);
      return saved ? parseInt(saved, 10) : DEFAULT_WIDTH;
    }
    return DEFAULT_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [newProjectName, setNewProjectName] = useState("New project");
  const [newProjectParent, setNewProjectParent] = useState("");
  const [newProjectFolder, setNewProjectFolder] = useState("project");
  const [newProjectError, setNewProjectError] = useState<string | null>(null);
  const [newProjectSubmitting, setNewProjectSubmitting] = useState(false);

  const sortedHistory = getHistory({ history });

  const filteredHistory = useMemo(() => {
    let list = sortedHistory;
    if (activeProjectId) {
      list = list.filter((item) => item.project_id === activeProjectId);
    } else {
      list = list.filter((item) => item.project_id == null);
    }
    if (!searchQuery.trim()) return list;
    const query = searchQuery.toLowerCase();
    return list.filter((item) => item.title.toLowerCase().includes(query));
  }, [sortedHistory, searchQuery, activeProjectId]);

  const visibleProjects = projects.slice(0, MAX_VISIBLE_PROJECTS);
  const overflowProjects = projects.slice(MAX_VISIBLE_PROJECTS);

  const toggleProjectExpanded = useCallback((id: string) => {
    setExpandedProjectIds((prev) => ({ ...prev, [id]: !prev[id] }));
  }, []);

  const handleChatClick = useCallback(
    (item: ChatHistoryItem) => {
      if (item.id === currentChatId) return;
      dispatch(restoreChat(item));
      dispatch(push({ name: "chat" }));
    },
    [dispatch, currentChatId],
  );

  const handleDeleteChat = useCallback(
    (id: string) => {
      dispatch(deleteChatById(id));
    },
    [dispatch],
  );

  const handleExportChat = useCallback((item: ChatHistoryItem) => {
    const html = chatToHtml(item);
    const blob = new Blob([html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `chat-${item.id}.html`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }, []);

  const handleCreateNewChat = useCallback(() => {
    const active = projects.find((p) => p.id === activeProjectId);
    const payload =
      active != null
        ? {
            project_id: active.id,
            esp32_projects_path: active.esp32_projects_path,
          }
        : undefined;
    const actions = [
      newChatAction(payload),
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
      popBackTo({ name: "history" }),
      push({ name: "chat" }),
    ];
    actions.forEach((action) => dispatch(action));
  }, [dispatch, projects, activeProjectId]);

  const openNewProjectDialog = useCallback(() => {
    const defaultName = "New project";
    const suggestedParent = defaultEspWorkspaceParentCandidates(
      currentThread.integration?.path,
    );
    setNewProjectName(defaultName);
    setNewProjectParent(suggestedParent);
    setNewProjectFolder(slugifyEspWorkspaceFolderName(defaultName));
    setNewProjectError(null);
    setNewProjectOpen(true);
  }, [currentThread.integration?.path]);

  const handleBrowseProjectParent = useCallback(async () => {
    const bridge = getCraftifDesktopBridge();
    if (typeof bridge?.browseFolder !== "function") return;
    try {
      const p = await bridge.browseFolder();
      if (p) setNewProjectParent(p);
    } catch {
      /* user cancelled or dialog error */
    }
  }, []);

  const submitNewProject = useCallback(async () => {
    const name = newProjectName.trim();
    const trimmedParent = newProjectParent.trim();
    const folder = newProjectFolder.trim();
    if (!name) {
      setNewProjectError("Project name is required.");
      return;
    }
    if (!trimmedParent) {
      setNewProjectError("Parent directory is required.");
      return;
    }
    if (!folder) {
      setNewProjectError("Folder name is required.");
      return;
    }
    setNewProjectError(null);
    setNewProjectSubmitting(true);
    const projectId = uuidv4();
    try {
      const { path } = await createEsp32ProjectWorkspace({
        parentPath: trimmedParent,
        folderName: folder,
        port: lspPort,
      });
      setDefaultEspWorkspaceParent(trimmedParent);
      dispatch(
        addProject({
          id: projectId,
          name,
          esp32_projects_path: path,
        }),
      );
      dispatch(
        newChatAction({
          project_id: projectId,
          esp32_projects_path: path,
        }),
      );
      dispatch(
        clearPauseReasonsAndHandleToolsStatus({
          wasInteracted: false,
          confirmationStatus: true,
        }),
      );
      dispatch(popBackTo({ name: "history" }));
      dispatch(push({ name: "chat" }));
      setNewProjectOpen(false);
    } catch (e) {
      setNewProjectError(e instanceof Error ? e.message : String(e));
    } finally {
      setNewProjectSubmitting(false);
    }
  }, [
    newProjectName,
    newProjectParent,
    newProjectFolder,
    lspPort,
    dispatch,
  ]);

  const selectProject = useCallback(
    (id: string) => {
      dispatch(setActiveProjectId(activeProjectId === id ? null : id));
    },
    [dispatch, activeProjectId],
  );

  const renderProjectRow = (p: WorkspaceProject) => {
    const expanded = !!expandedProjectIds[p.id];
    const isActive = activeProjectId === p.id;
    return (
      <Box key={p.id}>
        <Flex
          className={`${styles.projectRowWrap} ${isActive ? styles.projectRowWrapActive : ""}`}
          align="center"
          gap="0"
        >
          <IconButton
            type="button"
            size="1"
            variant="ghost"
            className={styles.projectChevronBtn}
            aria-label={expanded ? "Collapse project" : "Expand project"}
            onClick={(e) => {
              e.stopPropagation();
              toggleProjectExpanded(p.id);
            }}
          >
            {expanded ? (
              <ChevronDownIcon width="14" height="14" />
            ) : (
              <ChevronRightIcon width="14" height="14" />
            )}
          </IconButton>
          <Box
            role="button"
            tabIndex={0}
            className={styles.projectRow}
            onClick={() => selectProject(p.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                selectProject(p.id);
              }
            }}
          >
            <Flex align="center" gap="2" style={{ minWidth: 0, flex: 1 }}>
              <FiFolder size={16} aria-hidden />
              <Text size="2" className={styles.chatItemTitle} style={{ flex: 1, minWidth: 0 }}>
                {p.name}
              </Text>
            </Flex>
            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                <IconButton
                  size="1"
                  variant="ghost"
                  aria-label="Project options"
                  onClick={(e) => {
                    e.stopPropagation();
                  }}
                >
                  <DotsVerticalIcon width="14" height="14" />
                </IconButton>
              </DropdownMenu.Trigger>
              <DropdownMenu.Content side="bottom" align="end" size="1">
                <DropdownMenu.Item
                  onSelect={() => {
                    void (async () => {
                      const root = p.esp32_projects_path?.trim();
                      if (!root) {
                        dispatch(
                          setInformation(
                            "This project has no folder path yet. Create or select a project path first.",
                          ),
                        );
                        return;
                      }
                      try {
                        await openProjectSourcesInFileManager(root, lspPort);
                      } catch (err) {
                        dispatch(
                          setInformation(
                            `Could not open folder: ${
                              err instanceof Error ? err.message : String(err)
                            }`,
                          ),
                        );
                      }
                    })();
                  }}
                >
                  Open project folder
                </DropdownMenu.Item>
                <DropdownMenu.Item color="red" onSelect={() => dispatch(removeProject(p.id))}>
                  Remove from list
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </Box>
        </Flex>
        {expanded && (
          <Box pl="2" pb="1">
            <button
              type="button"
              className={styles.projectNestedRow}
              onClick={() => {
                dispatch(setActiveProjectId(p.id));
                dispatch(popBackTo({ name: "history" }));
                dispatch(push({ name: "chat" }));
              }}
            >
              Chats
            </button>
            <button
              type="button"
              className={styles.projectNestedRow}
              onClick={() => {
                dispatch(setActiveProjectId(p.id));
                dispatch(push({ name: "project sources", projectId: p.id }));
              }}
            >
              Sources
            </button>
          </Box>
        )}
      </Box>
    );
  };

  const handleNavigation = useCallback(
    (to: DropdownNavigationOptions) => {
      if (to === "settings") {
        openSettings();
      } else if (to === "hot keys") {
        openHotKeys();
      } else if (to === "fim") {
        dispatch(push({ name: "fill in the middle debug page" }));
      } else if (to === "stats") {
        dispatch(push({ name: "context payload page" }));
      } else if (to === "integrations") {
        dispatch(push({ name: "integrations page" }));
      } else if (to === "providers") {
        dispatch(push({ name: "providers page" }));
      }
    },
    [dispatch, openSettings, openHotKeys],
  );

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (panelRef.current) {
        const newWidth = e.clientX;
        const clamped = Math.max(220, Math.min(600, newWidth));
        setPanelWidth(clamped);
        try {
          localStorage.setItem(PANEL_WIDTH_KEY, clamped.toString());
        } catch {
          // ignore
        }
      }
    };

    const handleMouseUp = () => setIsResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing]);

  const showBrowseParent =
    typeof getCraftifDesktopBridge()?.browseFolder === "function";

  return (
    <>
      {isCollapsed ? (
        <Box className={styles.collapsedContainer}>
          <IconButton
            variant="ghost"
            onClick={() => setIsCollapsed(false)}
            title="Expand Chat History Panel"
          >
            <ChevronRightIcon />
          </IconButton>
        </Box>
      ) : (
    <Box
      ref={panelRef}
      className={styles.container}
      style={{ width: `${panelWidth}px` }}
    >
      <Flex className={styles.header} align="center" justify="end">
        <IconButton
          className={styles.headerCollapseButton}
          variant="ghost"
          size="2"
          onClick={() => setIsCollapsed(true)}
          title="Collapse Panel"
        >
          <ChevronLeftIcon />
        </IconButton>
      </Flex>

      <Box className={styles.searchContainer}>
        <TextField.Root
          className={styles.searchField}
          size="2"
          placeholder="Search chats..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        >
          <TextField.Slot>
            <MagnifyingGlassIcon height="14" width="14" />
          </TextField.Slot>
          {searchQuery && (
            <TextField.Slot>
              <IconButton
                size="1"
                variant="ghost"
                onClick={() => setSearchQuery("")}
              >
                <Cross1Icon height="12" width="12" />
              </IconButton>
            </TextField.Slot>
          )}
        </TextField.Root>
      </Box>
      <Box className={styles.newChatButtonContainer}>
        <Button
          className={styles.newChatButton}
          variant="outline"
          size="2"
          onClick={handleCreateNewChat}
        >
          <PlusIcon />
          <Text>New chat</Text>
        </Button>
      </Box>

      <Collapsible.Root
        className={styles.projectsCollapsibleRoot}
        open={projectsSectionExpanded}
        onOpenChange={(open) => dispatch(setProjectsSectionExpanded(open))}
      >
        <Collapsible.Trigger asChild>
          <button type="button" className={styles.projectsSectionTrigger}>
            <ChevronDownIcon
              width="14"
              height="14"
              className={`${styles.projectsChevron} ${projectsSectionExpanded ? styles.projectsChevronOpen : styles.projectsChevronClosed}`}
            />
            <Text as="span" weight="medium">
              Projects
            </Text>
          </button>
        </Collapsible.Trigger>
        <Collapsible.Content className={styles.projectsCollapsibleContent}>
          <button type="button" className={styles.newProjectRow} onClick={openNewProjectDialog}>
            <FiFolderPlus size={18} aria-hidden />
            <Text size="2">New project</Text>
          </button>
          {visibleProjects.map((p) => renderProjectRow(p))}
          {overflowProjects.length > 0 && (
            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                <button type="button" className={styles.moreProjectsRow}>
                  <Text size="2" color="gray" aria-hidden>
                    …
                  </Text>
                  <Text size="2">More</Text>
                </button>
              </DropdownMenu.Trigger>
              <DropdownMenu.Content side="bottom" align="start" size="2">
                {overflowProjects.map((p) => (
                  <DropdownMenu.Item
                    key={p.id}
                    onSelect={() => selectProject(p.id)}
                  >
                    <Flex align="center" gap="2">
                      <FiFolder size={14} />
                      {p.name}
                    </Flex>
                  </DropdownMenu.Item>
                ))}
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          )}
        </Collapsible.Content>
      </Collapsible.Root>

      <ProjectFileTree progressProjectPath={progressProjectPath} />

      <TopologyMinimapSection
        projectPath={progressProjectPath}
        currentStage={currentStage}
      />

      <button
        type="button"
        className={styles.projectsSectionTrigger}
        onClick={() => dispatch(setActiveProjectId(null))}
        aria-label="Recents"
        data-state="open"
        aria-expanded="true"
        title={activeProjectId ? "Show general chats" : "General chats"}
      >
        <span aria-hidden style={{ width: 14, height: 14, display: "inline-block" }} />
        <Text as="span" weight="medium">
          Recents
        </Text>
      </button>

      <Box className={styles.listContainer}>
        <ScrollArea scrollbars="vertical">
          {filteredHistory.length > 0 ? (
            <Box className={styles.chatList}>
              {filteredHistory.map((item) => {
                const isActive = item.id === currentChatId;

                return (
                  <Box
                    key={item.id}
                    className={`${styles.chatItem} ${isActive ? styles.chatItemActive : ""}`}
                    onClick={() => handleChatClick(item)}
                  >
                    <Flex align="center" gap="2" justify="between" mt="3px" mb="3px">
                      <Text className={styles.chatItemTitle}>{item.title}</Text>
                    </Flex>
                    <DropdownMenu.Root>
                      <DropdownMenu.Trigger>
                        <IconButton
                          className={styles.optionsButton}
                          size="1"
                          variant="ghost"
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                          }}
                          title="Chat options"
                          aria-label="Chat options"
                        >
                          <DotsVerticalIcon width="14" height="14" />
                        </IconButton>
                      </DropdownMenu.Trigger>
                      <DropdownMenu.Content
                        side="bottom"
                        align="end"
                        size="1"
                      >
                        {item.project_id == null && projects.length > 0 && (
                          <DropdownMenu.Sub>
                            <DropdownMenu.SubTrigger>
                              Move to project…
                            </DropdownMenu.SubTrigger>
                            <DropdownMenu.SubContent sideOffset={6} alignOffset={-6}>
                              {projects.map((p) => (
                                <DropdownMenu.Item
                                  key={p.id}
                                  onSelect={(event) => {
                                    event.preventDefault();
                                    event.stopPropagation();
                                    dispatch(
                                      setChatProjectById({
                                        chatId: item.id,
                                        projectId: p.id,
                                        projectName: p.name,
                                        esp32_projects_path: p.esp32_projects_path,
                                      }),
                                    );
                                    dispatch(
                                      setChatProject({
                                        chatId: item.id,
                                        projectId: p.id,
                                        projectName: p.name,
                                        esp32_projects_path: p.esp32_projects_path,
                                      }),
                                    );
                                  }}
                                >
                                  {p.name}
                                </DropdownMenu.Item>
                              ))}
                            </DropdownMenu.SubContent>
                          </DropdownMenu.Sub>
                        )}
                        <DropdownMenu.Item
                          onSelect={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            handleExportChat(item);
                          }}
                        >
                          Export chat
                        </DropdownMenu.Item>
                        <DropdownMenu.Item
                          color="red"
                          onSelect={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            handleDeleteChat(item.id);
                          }}
                        >
                          Delete chat
                        </DropdownMenu.Item>
                      </DropdownMenu.Content>
                    </DropdownMenu.Root>
                  </Box>
                );
              })}
            </Box>
          ) : (
            <Box className={styles.emptyState}>
              <ChatBubbleIcon className={styles.emptyStateIcon} />
              <Text size="2" color="gray">
                {searchQuery
                  ? "No chats match your search"
                  : activeProjectId
                    ? "No chats in this project yet. Use New chat or open Chats from the project menu."
                    : "No chat history yet. Start a conversation!"}
              </Text>
            </Box>
          )}
        </ScrollArea>
      </Box>

      <Box className={styles.profileFooter}>
        <Dropdown
          handleNavigation={handleNavigation}
          trigger={
            <button type="button" className={styles.profileButton}>
              <Flex align="center" justify="between" gap="3">
                <Flex align="center" gap="3" className={styles.profileLeft}>
                  <Box className={styles.profileAvatar} aria-hidden>
                    {profileInitial}
                  </Box>
                  <Flex direction="column" className={styles.profileText}>
                    <Text
                      size="2"
                      weight="medium"
                      className={styles.profileName}
                    >
                      {profileName}
                    </Text>
                    <Text size="1" color="gray" className={styles.profilePlan}>
                      Pro Plan
                    </Text>
                  </Flex>
                </Flex>
              </Flex>
            </button>
          }
        />
      </Box>

      <Box
        className={styles.resizeHandle}
        onMouseDown={handleMouseDown}
        style={{ cursor: isResizing ? "col-resize" : "ew-resize" }}
        title="Drag to resize panel"
      >
        <DragHandleVerticalIcon />
      </Box>
    </Box>
      )}

      <Dialog.Root
        open={newProjectOpen}
        onOpenChange={(open) => {
          setNewProjectOpen(open);
          if (!open) {
            setNewProjectSubmitting(false);
            setNewProjectError(null);
          }
        }}
      >
        <Dialog.Content maxWidth="480px">
          <Dialog.Title>New ESP32 project</Dialog.Title>
          <Dialog.Description size="2" mb="3" color="gray">
            Choose a display name, parent folder, and on-disk folder name (letters, digits,
            . _ - only).
          </Dialog.Description>

          <Flex direction="column" gap="3">
            <Flex direction="column" gap="1">
              <Text size="2" weight="medium" as="label" htmlFor="new-project-name">
                Project name
              </Text>
              <TextField.Root
                id="new-project-name"
                size="2"
                value={newProjectName}
                onChange={(e) => setNewProjectName(e.target.value)}
                placeholder="My firmware"
              />
            </Flex>

            <Flex direction="column" gap="1">
              <Text size="2" weight="medium" as="label" htmlFor="new-project-parent">
                Parent directory (absolute path)
              </Text>
              <Flex gap="2" align="center">
                <TextField.Root
                  id="new-project-parent"
                  size="2"
                  className={styles.newProjectParentField}
                  value={newProjectParent}
                  onChange={(e) => setNewProjectParent(e.target.value)}
                  placeholder="C:\Users\you\esp-projects"
                />
                {showBrowseParent ? (
                  <Button
                    type="button"
                    variant="soft"
                    size="2"
                    onClick={() => void handleBrowseProjectParent()}
                  >
                    Browse…
                  </Button>
                ) : null}
              </Flex>
            </Flex>

            <Flex direction="column" gap="1">
              <Text size="2" weight="medium" as="label" htmlFor="new-project-folder">
                Folder name on disk
              </Text>
              <TextField.Root
                id="new-project-folder"
                size="2"
                value={newProjectFolder}
                onChange={(e) => setNewProjectFolder(e.target.value)}
                placeholder="my_firmware"
              />
            </Flex>

            {newProjectError ? (
              <Text size="2" color="red">
                {newProjectError}
              </Text>
            ) : null}
          </Flex>

          <Flex gap="3" justify="end" mt="4">
            <Dialog.Close>
              <Button variant="soft" color="gray" type="button" disabled={newProjectSubmitting}>
                Cancel
              </Button>
            </Dialog.Close>
            <Button
              type="button"
              disabled={newProjectSubmitting}
              onClick={() => void submitNewProject()}
            >
              {newProjectSubmitting ? "Creating…" : "Create project"}
            </Button>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>
    </>
  );
};
