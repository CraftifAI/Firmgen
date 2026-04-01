import {
  Flex,
  TabNav,
  TextField,
} from "@radix-ui/themes";
import type { DropdownNavigationOptions } from "./Dropdown";
import Sharebutton from "./Sharebutton";
import { newChatAction } from "../../events";
import { restart, useTourRefs } from "../../features/Tour";
import { popBackTo, push } from "../../features/Pages/pagesSlice";
import {
  ChangeEvent,
  KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  getHistory,
  updateChatTitleById,
} from "../../features/History/historySlice";
import { restoreChat, saveTitle, selectThread } from "../../features/Chat";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
} from "../../hooks";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { clearPauseReasonsAndHandleToolsStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { telemetryApi } from "../../services/refact/telemetry";
import { selectEmbedded } from "../../features/Config/configSlice";

import styles from "./Toolbar.module.css";
import { useActiveTeamsGroup } from "../../hooks/useActiveTeamsGroup";
import {
  resetActiveGroup,
  resetActiveWorkspace,
  setSkippedWorkspaceSelection,
} from "../../features/Teams";

export type DashboardTab = {
  type: "dashboard";
};

function isDashboardTab(tab: Tab): tab is DashboardTab {
  return tab.type === "dashboard";
}

export type ChatTab = {
  type: "chat";
  id: string;
};

export type Tab = DashboardTab | ChatTab;

export type ToolbarProps = {
  activeTab: Tab;
};

export const Toolbar = ({ activeTab }: ToolbarProps) => {
  const dispatch = useAppDispatch();
  const { width: windowWidth } = useWindowDimensions();

  const refs = useTourRefs();
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const history = useAppSelector(getHistory, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const { isTitleGenerated, id: chatId } = useAppSelector(selectThread);
  const { newChatEnabled } = useActiveTeamsGroup();
  const hasEmbeddedFeatures = useAppSelector(selectEmbedded);

  const { openSettings, openHotKeys } = useEventsBusForIDE();

  const [isOnlyOneChatTab, setIsOnlyOneChatTab] = useState(false);
  const [isRenaming, setIsRenaming] = useState(false);
  const [newTitle, setNewTitle] = useState<string | null>(null);

  const shouldChatTabLinkBeNotClickable = useMemo(() => {
    return isOnlyOneChatTab && !isDashboardTab(activeTab);
  }, [isOnlyOneChatTab, activeTab]);

  const handleNavigation = useCallback(
    (to: DropdownNavigationOptions | "chat") => {
      if (to === "settings") {
        openSettings();
        void sendTelemetryEvent({
          scope: `openSettings`,
          success: true,
          error_message: "",
        });
      } else if (to === "hot keys") {
        openHotKeys();
        void sendTelemetryEvent({
          scope: `openHotkeys`,
          success: true,
          error_message: "",
        });
      } else if (to === "fim") {
        dispatch(push({ name: "fill in the middle debug page" }));
        void sendTelemetryEvent({
          scope: `openDebugFim`,
          success: true,
          error_message: "",
        });
      } else if (to === "stats") {
        dispatch(push({ name: "statistics page" }));
        void sendTelemetryEvent({
          scope: `openStats`,
          success: true,
          error_message: "",
        });
      } else if (to === "restart tour") {
        dispatch(popBackTo({ name: "login page" }));
        dispatch(push({ name: "welcome" }));
        dispatch(restart());
        void sendTelemetryEvent({
          scope: `restartTour`,
          success: true,
          error_message: "",
        });
      } else if (to === "integrations") {
        dispatch(push({ name: "integrations page" }));
        void sendTelemetryEvent({
          scope: `openIntegrations`,
          success: true,
          error_message: "",
        });
      } else if (to === "providers") {
        dispatch(push({ name: "providers page" }));
        void sendTelemetryEvent({
          scope: `openProviders`,
          success: true,
          error_message: "",
        });
      } else if (to === "chat") {
        dispatch(popBackTo({ name: "history" }));
        dispatch(push({ name: "chat" }));
      }
    },
    [dispatch, sendTelemetryEvent, openSettings, openHotKeys],
  );

  const onCreateNewChat = useCallback(() => {
    setIsRenaming((prev) => (prev ? !prev : prev));
    dispatch(newChatAction());
    dispatch(
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
    );
    handleNavigation("chat");
    void sendTelemetryEvent({
      scope: `openNewChat`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent, handleNavigation]);

  const goToWorkspacePathUi = useCallback(() => {
    setIsRenaming((prev) => (prev ? !prev : prev));
    dispatch(setSkippedWorkspaceSelection(false));
    dispatch(resetActiveGroup());
    dispatch(resetActiveWorkspace());
    dispatch(popBackTo({ name: "history" }));
    void sendTelemetryEvent({
      scope: `goToWorkspacePathUi`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent]);

  const goToTab = useCallback(
    (tab: Tab) => {
      if (tab.type === "dashboard") {
        dispatch(popBackTo({ name: "history" }));
        dispatch(newChatAction());
      } else {
        if (shouldChatTabLinkBeNotClickable) return;
        const chat = history.find((chat) => chat.id === tab.id);
        if (chat != undefined) {
          dispatch(restoreChat(chat));
        }
        dispatch(popBackTo({ name: "history" }));
        dispatch(push({ name: "chat" }));
      }
      void sendTelemetryEvent({
        scope: `goToTab/${tab.type}`,
        success: true,
        error_message: "",
      });
    },
    [dispatch, history, shouldChatTabLinkBeNotClickable, sendTelemetryEvent],
  );

  const tabs = useMemo(() => {
    return history.filter(
      (chat) =>
        chat.read === false ||
        (activeTab.type === "chat" && activeTab.id == chat.id),
    );
  }, [history, activeTab]);

  const handleKeyUpOnRename = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.code === "Escape") {
        setIsRenaming(false);
      }
      if (event.code === "Enter") {
        setIsRenaming(false);
        if (!newTitle || newTitle.trim() === "") return;
        if (!isTitleGenerated) {
          dispatch(
            saveTitle({
              id: chatId,
              title: newTitle,
              isTitleGenerated: true,
            }),
          );
        }
        dispatch(updateChatTitleById({ chatId: chatId, newTitle: newTitle }));
      }
    },
    [dispatch, newTitle, chatId, isTitleGenerated],
  );

  const handleChatTitleChange = (event: ChangeEvent<HTMLInputElement>) => {
    setNewTitle(event.target.value);
  };

  useEffect(() => {
    setIsOnlyOneChatTab(tabs.length < 2);
  }, [tabs]);

  return (
    <Flex align="end" m="4px" gap="4px" style={{ alignSelf: "left" }}>
      <Flex flexGrow="1" align="end" style={{ minHeight: 70 }}>
        <TabNav.Root
          style={{ flex: 1, overflowY: "visible" }}
          className={styles.tabNavRootWithLogo}
        >
          <TabNav.Link
            active={isDashboardTab(activeTab)}
            ref={(x) => refs.setBack(x)}
            onClick={() => {
              goToWorkspacePathUi();
            }}
            style={{
              width: "fit-content",
              minWidth: 0,
              height: "fit-content",
              maxWidth: "100%",
              overflow: "hidden",
            }}
            className={styles.homeTabWithLogo}
          >
            <img
              src="/new_logo.png"
              alt="Workspace setup"
              width={50}
              height={60}
              style={{ objectFit: "contain", cursor: "pointer" }}
              title="Workspace and folder path"
            />
          </TabNav.Link>
          <TabNav.Link
            active={false}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();

            }}
            style={{
              width: "fit-content",
              minWidth: 0,
              cursor: "default",
              pointerEvents: "auto",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              height: "inherit",
              maxHeight: "inherit",
            }}
            className={styles.homeLabelTab}
          >
            <span
              style={{
                fontFamily: "'Anthropic', serif",
                fontSize: 27,
                fontWeight: "normal",
                whiteSpace: "nowrap",
                textAlign: "left",
                letterSpacing: "2.15px",
                color: "var(--palette-text-main)",
                margin: 0,
              }}
            >
              FirmGen
            </span>
          </TabNav.Link>
          {tabs.map((chat) => {
            if (isRenaming) {
              return (
                <TextField.Root
                  my="auto"
                  key={chat.id}
                  autoComplete="off"
                  onKeyUp={handleKeyUpOnRename}
                  onBlur={() => setIsRenaming(false)}
                  autoFocus
                  size="2"
                  defaultValue={isTitleGenerated ? chat.title : ""}
                  onChange={handleChatTitleChange}
                  className={styles.RenameInput}
                />
              );
            }
            return null;
            // return (
            // <TabNav.Link
            //   active={isActive}
            //   key={chat.id}
            //   onClick={() => {
            //     if (shouldChatTabLinkBeNotClickable) return;
            //     goToTab({ type: "chat", id: chat.id });
            //   }}
            //   style={{ minWidth: 0, maxWidth: "150px", cursor: "pointer" }}
            //   ref={isActive ? setFocus : undefined}
            //   title={chat.title}
            // >
            //   {isStreamingThisTab && <Spinner />}
            //   {!isStreamingThisTab && chat.read === false && (
            //     <DotFilledIcon />
            //   )}
            //   <Flex gap="2" align="center">
            //     <TruncateLeft
            //       style={{
            //         maxWidth: shouldCollapse ? "25px" : "110px",
            //       }}
            //     >
            //       {chat.title}
            //     </TruncateLeft>
            //     {isActive && !isStreamingThisTab && isOnlyOneChatTab && (
            //       <DropdownMenu.Root>
            //         <DropdownMenu.Trigger>
            //           <IconButton
            //             size="1"
            //             variant="ghost"
            //             color="gray"
            //             title="Title actions"
            //           >
            //             <DotsVerticalIcon />
            //           </IconButton>
            //         </DropdownMenu.Trigger>
            //         <DropdownMenu.Content
            //           size="1"
            //           side="bottom"
            //           align="end"
            //           style={{
            //             minWidth: 110,
            //           }}
            //         >
            //           <DropdownMenu.Item onClick={handleChatThreadRenaming}>
            //             Rename
            //           </DropdownMenu.Item>
            //           <DropdownMenu.Item
            //             onClick={handleChatThreadDeletion}
            //             color="red"
            //           >
            //             Delete chat
            //           </DropdownMenu.Item>
            //         </DropdownMenu.Content>
            //       </DropdownMenu.Root>
            //     )}
            //   </Flex>
            // </TabNav.Link>
            // );
          })}
        </TabNav.Root>
      </Flex>
      {/* {!hasEmbeddedFeatures &&
        (windowWidth < 400 ? (
          <IconButton
            variant="outline"
            ref={(x) => refs.setNewChat(x)}
            onClick={onCreateNewChat}
          >
            <PlusIcon />
          </IconButton>
        ) : (
          <Button
            variant="outline"
            ref={(x) => refs.setNewChat(x)}
            onClick={onCreateNewChat}
            disabled={!newChatEnabled}
          >
            <PlusIcon />
            <Text>New chat</Text>
          </Button>
        ))} */}
      <Sharebutton chatId={chatId} />
    </Flex>
  );
};
