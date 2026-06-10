import React, { useCallback, useState } from "react";
import { ChatForm, ChatFormProps } from "../ChatForm";
import { ChatContent } from "../ChatContent";
import { PlaceHolderText } from "../ChatContent/PlaceHolderText";
import { Flex, Button, Text, Card } from "@radix-ui/themes";
import {
  useAppSelector,
  useAppDispatch,
  useSendChatRequest,
  useAutoSend,
  useCapsForToolUse,
  useProgress,
} from "../../hooks";
import { type Config } from "../../features/Config/configSlice";
import {
  enableSend,
  selectIsStreaming,
  selectPreventSend,
  selectChatId,
  selectMessages,
  getSelectedToolUse,
  selectThreadNewChatSuggested,
} from "../../features/Chat/Thread";
import { ThreadHistoryButton } from "../Buttons";
import { push } from "../../features/Pages/pagesSlice";
import { DropzoneProvider } from "../Dropzone";
import { useCheckpoints } from "../../hooks/useCheckpoints";
import { Checkpoints } from "../../features/Checkpoints";
import { SuggestNewChat } from "../ChatForm/SuggestNewChat";
import { DevicePanels } from "../EmbeddedPanels";
// ContextPayloadSidebar is disabled in favor of ChatHistorySidebar
// import { ContextPayloadSidebar } from "../ContextPayloadSidebar";
import { ChatHistorySidebar } from "../ChatHistorySidebar";
import { selectEmbedded } from "../../features/Config/configSlice";
import { ProgressBarSafe as ProgressBar } from "../ProgressBar";
import { ComposerQuickActions } from "./ComposerQuickActions";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromChat: () => void;
  style?: React.CSSProperties;
  unCalledTools: boolean;
  maybeSendToSidebar: ChatFormProps["onClose"];
};

export const Chat: React.FC<ChatProps> = ({
  style,
  unCalledTools,
  maybeSendToSidebar,
}) => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const messages = useAppSelector(selectMessages);
  const progressApi = useProgress({
    chatId: chatId ?? undefined,
    pollingInterval: 500,
  });
  const isStreaming = useAppSelector(selectIsStreaming);

  // Progress is driven solely by backend /v1/progress (engine progressbar module)
  const currentStage = progressApi.currentStage;
  const hasError = progressApi.hasError;
  const isProgressActive = isStreaming || progressApi.isStreaming;

  const [isViewingRawJSON, setIsViewingRawJSON] = useState(false);

  const { submit, abort, retryFromIndex } = useSendChatRequest();

  const chatToolUse = useAppSelector(getSelectedToolUse);
  const threadNewChatSuggested = useAppSelector(selectThreadNewChatSuggested);
  const capsForToolUse = useCapsForToolUse();

  const { shouldCheckpointsPopupBeShown } = useCheckpoints();

  const [isDebugChatHistoryVisible, setIsDebugChatHistoryVisible] =
    useState(false);

  const preventSend = useAppSelector(selectPreventSend);
  const onEnableSend = () => dispatch(enableSend({ id: chatId }));
  const hasEmbeddedFeatures = useAppSelector(selectEmbedded);

  // Debug logging
  React.useEffect(() => {
    console.log("Chat component - hasEmbeddedFeatures:", hasEmbeddedFeatures);
  }, [hasEmbeddedFeatures]);

  // Keep a stable ref to resetForNewRun so handleSummit doesn't recreate on
  // every render (progressApi is a new object every render and must not be in
  // the callback dependency array).
  const resetForNewRunRef = React.useRef(progressApi.resetForNewRun);
  React.useEffect(() => {
    resetForNewRunRef.current = progressApi.resetForNewRun;
  });

  const handleSummit = useCallback(
    (value: string) => {
      // Clear stale progress so the bar resets before new tool events arrive.
      try { resetForNewRunRef.current?.(); } catch (_) { /* never block submit */ }
      submit({ question: value });
      if (isViewingRawJSON) {
        setIsViewingRawJSON(false);
      }
    },
    [submit, isViewingRawJSON],
  );

  const handleThreadHistoryPage = useCallback(() => {
    dispatch(push({ name: "thread history page", chatId }));
  }, [chatId, dispatch]);





  useAutoSend();


  const isEmpty = messages.length === 0;

  const chatContent = isEmpty ? (
    <>
      <Flex
        direction="column"
        align="center"
        justify="center"
        flexGrow="1"
        gap="4"
        style={{ minHeight: 0, width: "100%" }}
      >
        <PlaceHolderText />
        <Flex direction="column" align="center" style={{ width: "100%", maxWidth: "700px" }}>
          <ChatForm
            key={chatId}
            onSubmit={handleSummit}
            onClose={maybeSendToSidebar}
            unCalledTools={unCalledTools}
          />
        </Flex>
        <ComposerQuickActions disabled={isStreaming}/>
        <Text size="1" style={{ color: "var(--gray-9)" }}>
          FirmGen may make mistakes. Verify important outputs.
        </Text>
      </Flex>
      {hasEmbeddedFeatures && <DevicePanels />}
    </>
  ) : (
    <>
      <ProgressBar
        currentStage={currentStage}
        isStreaming={isProgressActive}
        hasError={hasError}
        isDebugging={progressApi.isDebugging}
        debugIteration={progressApi.debugIteration}
        events={progressApi.progress?.events}
      />
      <ChatContent
        key={`chat-content-${chatId}`}
        onRetry={retryFromIndex}
        onStopStreaming={abort}
      />

      {shouldCheckpointsPopupBeShown && <Checkpoints />}

      <SuggestNewChat
        shouldBeVisible={
          threadNewChatSuggested.wasSuggested &&
          !threadNewChatSuggested.wasRejectedByUser
        }
      />
      {!isStreaming && preventSend && unCalledTools && (
        <Flex py="4">
          <Card style={{ width: "100%" }}>
            <Flex direction="column" align="center" gap="2" width="100%">
              Chat was interrupted with uncalled tools calls.
              <Button onClick={onEnableSend}>Resume</Button>
            </Flex>
          </Card>
        </Flex>
      )}

      <ChatForm
        key={chatId}
        onSubmit={handleSummit}
        onClose={maybeSendToSidebar}
        unCalledTools={unCalledTools}
      />

      {hasEmbeddedFeatures && <DevicePanels />}

      <Flex justify="between" pl="1" pr="1" pt="1">
      </Flex>
    </>
  );

  if (!hasEmbeddedFeatures) {
    // Original structure when embedded features are disabled
    return (
      <DropzoneProvider asChild>
        <Flex
          style={style}
          direction="column"
          flexGrow="1"
          minHeight="0"
          width="100%"
          overflowY="auto"
          justify="between"
          px="1"
        >
          {chatContent}
        </Flex>
      </DropzoneProvider>
    );
  }

  // New structure with left context panel and right embedded panels
  return (
    <DropzoneProvider asChild>
      <Flex
        style={style}
        direction="row"
        flexGrow="1"
        minHeight="0"
        width="100%"
        height="100%"
      >
        {/* Left Chat History Panel (replaces ContextPayloadSidebar) */}
        <ChatHistorySidebar
          progressProjectPath={progressApi.progress?.esp32_project_path ?? null}
        />
        {/* ContextPayloadSidebar disabled — kept for future use:
        <React.Suspense fallback={<div>Loading context panel...</div>}>
          <ContextPayloadSidebar />
        </React.Suspense>
        */}

        {/* Main chat content */}
        <Flex
          direction="column"
          flexGrow="1"
          width="100%"
          minHeight="0"
          minWidth="0"
          overflowY="auto"
          justify="between"
          px="1"
        >
          {chatContent}
        </Flex>

      </Flex>
    </DropzoneProvider>
  );
};
