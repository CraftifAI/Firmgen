import React, { useState, useCallback, useEffect, useRef, useMemo } from "react";
import {
  Box,
  Flex,
  Text,
  IconButton,
  Button,
  TextField,
  DropdownMenu,
} from "@radix-ui/themes";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ChatBubbleIcon,
  MagnifyingGlassIcon,
  Cross1Icon,
  DotsVerticalIcon,
  DragHandleVerticalIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
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
  type ChatHistoryItem,
} from "../../features/History/historySlice";
import { restoreChat, selectChatId } from "../../features/Chat/Thread";
import { popBackTo, push } from "../../features/Pages/pagesSlice";
import { ScrollArea } from "../ScrollArea";
import styles from "./ChatHistorySidebar.module.css";
import { newChatAction } from "../../events";
import { clearPauseReasonsAndHandleToolsStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { Dropdown, type DropdownNavigationOptions } from "../Toolbar/Dropdown";

const PANEL_WIDTH_KEY = "chatHistorySidebarWidth";
const DEFAULT_WIDTH = 320;

export const ChatHistorySidebar: React.FC = () => {
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const { displayName: craftifDisplayName } = useCraftifAuth();
  const profileName =
    craftifDisplayName?.trim() ||
    user.data?.fuser_id ||
    "User";
  const profileInitial = profileName.trim().charAt(0).toUpperCase() || "R";
  const { openSettings, openHotKeys } = useEventsBusForIDE();
  const history = useAppSelector((app) => app.history, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const currentChatId = useAppSelector(selectChatId);

  const [isCollapsed, setIsCollapsed] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  const [panelWidth, setPanelWidth] = useState(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(PANEL_WIDTH_KEY);
      return saved ? parseInt(saved, 10) : DEFAULT_WIDTH;
    }
    return DEFAULT_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const sortedHistory = getHistory({ history });

  const filteredHistory = useMemo(() => {
    if (!searchQuery.trim()) return sortedHistory;
    const query = searchQuery.toLowerCase();
    return sortedHistory.filter((item) =>
      item.title.toLowerCase().includes(query),
    );
  }, [sortedHistory, searchQuery]);

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
    // Exporting locally by downloading JSON of the chat history item.
    const payload = {
      ...item,
      exportedAt: new Date().toISOString(),
    };

    const blob = new Blob([JSON.stringify(payload, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `chat-${item.id}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }, []);

  const handleCreateNewChat = useCallback(() => {
    const actions = [
      newChatAction(),
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
      popBackTo({ name: "history" }),
      push({ name: "chat" }),
    ];
    actions.forEach((action) => dispatch(action));
  }, [dispatch]);

  const handleNavigation = useCallback(
    (to: DropdownNavigationOptions) => {
      if (to === "settings") {
        openSettings();
      } else if (to === "hot keys") {
        openHotKeys();
      } else if (to === "fim") {
        dispatch(push({ name: "fill in the middle debug page" }));
      } else if (to === "stats") {
        dispatch(push({ name: "statistics page" }));
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

  // (formatDate and some per-item vars were previously used for meta rows;
  // keeping sidebar lean for now.)

  if (isCollapsed) {
    return (
      <Box className={styles.collapsedContainer}>
        <IconButton
          variant="ghost"
          onClick={() => setIsCollapsed(false)}
          title="Expand Chat History Panel"
        >
          <ChevronRightIcon />
        </IconButton>
      </Box>
    );
  }

  return (
    <Box
      ref={panelRef}
      className={styles.container}
      style={{ width: `${panelWidth}px` }}
    >
      <Flex
        className={styles.header}
        align="center"
      >
        <Flex align="center" gap="2" className={styles.headerTitleRow}>
          <Text weight="bold" size="3" className={styles.headerTitle}>
            Recents
          </Text>
          {/* <Badge className={styles.headerBadge} size="2">
            {sortedHistory.length}
          </Badge> */}
        </Flex>
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

      <Box className={styles.listContainer}>
        <ScrollArea scrollbars="vertical">
          {filteredHistory.length > 0 ? (
            <Box className={styles.chatList}>
              {filteredHistory.map((item) => {
                const isActive = item.id === currentChatId;
                // const isStreaming = item.id in cache;
                // const userMsgCount = item.messages.filter(isUserMessage).length;

                return (
                  <Box
                    key={item.id}
                    className={`${styles.chatItem} ${isActive ? styles.chatItemActive : ""}`}
                    onClick={() => handleChatClick(item)}
                  >
                    <Flex align="center" gap="2" justify="between" mt="3px" mb="3px">
                      {/* {isStreaming && <span className={styles.streamingIndicator} />}
                      {!isStreaming && item.read === false && (
                        <span className={styles.unreadDot} />
                      )} */}
                      <Text className={styles.chatItemTitle}>{item.title}</Text>
                    </Flex>
                    {/* <Flex className={styles.chatItemMeta}>
                      <span className={styles.chatItemMessageCount}>
                        <ChatBubbleIcon width="11" height="11" />
                        {userMsgCount}
                      </span>
                      <span className={styles.chatItemDate}>
                        {formatDate(item.updatedAt)}
                      </span>
                    </Flex> */}
                    <DropdownMenu.Root >
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
                        <DropdownMenu.Item
                          onSelect={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            handleExportChat(item);
                          }}
                        >
                          Export chat
                        </DropdownMenu.Item>
                        {/* <DropdownMenu.Separator /> */}
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
  );
};






