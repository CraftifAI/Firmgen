import React, { useCallback } from "react";
import { Box, Flex, Spinner, Button } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  ChatHistoryItem,
  deleteChatById,
} from "../../features/History/historySlice";
import { push } from "../../features/Pages/pagesSlice";
import { restoreChat } from "../../features/Chat/Thread";
import { FeatureMenu } from "../../features/Config/FeatureMenu";
import { GroupTree } from "./GroupTree/";
import { ErrorCallout } from "../Callout";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import classNames from "classnames";
import { selectHost } from "../../features/Config/configSlice";
import styles from "./Sidebar.module.css";
import { useActiveTeamsGroup } from "../../hooks/useActiveTeamsGroup";
import { useCraftifAuth } from "../../hooks";

export type SidebarProps = {
  takingNotes: boolean;
  className?: string;
  style?: React.CSSProperties;
} & Omit<
  ChatHistoryProps,
  | "history"
  | "onDeleteHistoryItem"
  | "onCreateNewChat"
  | "onHistoryItemClick"
  | "currentChatId"
>;

export const Sidebar: React.FC<SidebarProps> = ({ takingNotes, style }) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const globalError = useAppSelector(getErrorMessage);
  const currentHost = useAppSelector(selectHost);
  const history = useAppSelector((app) => app.history, {
    // TODO: selector issue here
    devModeChecks: { stabilityCheck: "never" },
  });

  const { groupSelectionEnabled } = useActiveTeamsGroup();
  const { user } = useCraftifAuth();

  const onDeleteHistoryItem = useCallback(
    (id: string) => dispatch(deleteChatById(id)),
    [dispatch],
  );

  const onHistoryItemClick = useCallback(
    (thread: ChatHistoryItem) => {
      dispatch(restoreChat(thread));
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  return (
    <Flex style={style} direction="column">
      <FeatureMenu />
      <Flex mt="4" mb="4" justify="center">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>

      {user?.role === "ADMIN" && (
        <Box px="4" py="2">
          <Button
            variant="soft"
            color="indigo"
            style={{ width: "100%" }}
            onClick={() => dispatch(push({ name: "admin usage page" }))}
          >
            Admin Panel
          </Button>
        </Box>
      )}

      {!groupSelectionEnabled ? (
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      ) : (
        <GroupTree />
      )}
      {/* TODO: duplicated */}
      {globalError && (
        <ErrorCallout
          mx="0"
          timeout={3000}
          onClick={() => dispatch(clearError())}
          className={classNames(styles.popup, {
            [styles.popup_ide]: currentHost !== "web",
          })}
          preventRetry
        >
          {globalError}
        </ErrorCallout>
      )}
    </Flex>
  );
};
