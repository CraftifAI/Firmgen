import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card, Text, IconButton } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import {
  BackToSideBarButton,
  AgentIntegrationsButton
  // ThinkingButton,
} from "../Buttons";
import { TextArea } from "../TextArea";
import { IoIosSend } from "react-icons/io";
import { IoStop } from "react-icons/io5";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  useIsOnline,
  useConfig,
  useCapsForToolUse,
  useSendChatRequest,
  useCompressChat,
  useAutoFocusOnce,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { ComboBox } from "../ComboBox";
import { FilesPreview } from "./FilesPreview";
import { CapsSelect, ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { useCommandCompletionAndPreviewFiles } from "./useCommandCompletionAndPreviewFiles";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { parseFiles, type ParsedFile } from "../../services/fileParser";
import {
  setPendingProcessedContext,
  setLastUserMessageDisplayContent,
} from "../../features/Chat/Thread/actions";
import {
  clearError,
  getErrorMessage,
  getErrorType,
} from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useAttachedFiles, useCheckboxes } from "./useCheckBoxes";
import { useInputValue } from "./useInputValue";
import {
  clearInformation,
  getInformationMessage,
  showBalanceLowCallout,
} from "../../features/Errors/informationSlice";
import {
  BallanceCallOut,
  BallanceLowInformation,
  InformationCallout,
} from "../Callout/Callout";
import { ToolConfirmation } from "./ToolConfirmation";
import { getPauseReasonsWithPauseStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { AttachImagesButton, FileList } from "../Dropzone";
import { useAttachedImages } from "../../hooks/useAttachedImages";
import {
  selectChatError,
  selectIsStreaming,
  selectIsWaiting,
  selectLastSentCompression,
  selectMessages,
  selectThreadToolUse,
  selectToolUse,
} from "../../features/Chat";
import { telemetryApi } from "../../services/refact";
import { push } from "../../features/Pages/pagesSlice";
import { AgentCapabilities } from "./AgentCapabilities/AgentCapabilities";
import { TokensPreview } from "./TokensPreview";
import classNames from "classnames";
import {
  ArchiveIcon,
  FilePlusIcon,
  Cross2Icon,
  PaperPlaneIcon,
} from "@radix-ui/react-icons";
import craftifPanelBtn from "../CraftifPanelButton/craftifPanelButton.module.css";

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  unCalledTools: boolean;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
  unCalledTools,
}) => {
  const dispatch = useAppDispatch();
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
  const config = useConfig();
  const toolUse = useAppSelector(selectToolUse);
  const globalError = useAppSelector(getErrorMessage);
  const globalErrorType = useAppSelector(getErrorType);
  const chatError = useAppSelector(selectChatError);
  const information = useAppSelector(getInformationMessage);
  const pauseReasonsWithPause = useAppSelector(getPauseReasonsWithPauseStatus);
  const [helpInfo, setHelpInfo] = React.useState<React.ReactNode | null>(null);
  const isOnline = useIsOnline();
  const { retry, abort } = useSendChatRequest();

  const threadToolUse = useAppSelector(selectThreadToolUse);
  const messages = useAppSelector(selectMessages);
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  // const { compressChat, compressChatRequest, isCompressing } =
  //   useCompressChat();
  const autoFocus = useAutoFocusOnce();
  const attachedFiles = useAttachedFiles();
  const shouldShowBalanceLow = useAppSelector(showBalanceLowCallout);

  const shouldAgentCapabilitiesBeShown = useMemo(() => {
    return threadToolUse === "agent";
  }, [threadToolUse]);

  const onClearError = useCallback(() => {
    if (messages.length > 0 && chatError) {
      retry(messages);
    }
    dispatch(clearError());
  }, [dispatch, retry, messages, chatError]);

  const caps = useCapsForToolUse();

  const allDisabled = caps.usableModelsForPlan.every((option) => {
    if (typeof option === "string") return false;
    return option.disabled;
  });

  const disableSend = useMemo(() => {
    // TODO: if interrupting chat some errors can occur
    if (allDisabled) return true;
    // if (
    //   currentThreadMaximumContextTokens &&
    //   currentThreadUsage?.prompt_tokens &&
    //   currentThreadUsage.prompt_tokens > currentThreadMaximumContextTokens
    // )
    //   return false;
    // if (arePromptTokensBiggerThanContext) return true;
    if (messages.length === 0) return false;
    return isWaiting || isStreaming || !isOnline;
  }, [allDisabled, messages.length, isWaiting, isStreaming, isOnline]);

  // const isModelSelectVisible = useMemo(() => messages.length < 1, [messages]);

  const { processAndInsertImages } = useAttachedImages();
  const handlePastingFile = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      if (!isMultimodalitySupportedForCurrentModel) return;
      const files: File[] = [];
      const items = event.clipboardData.items;
      for (const item of items) {
        if (item.kind === "file") {
          const file = item.getAsFile();
          file && files.push(file);
        }
      }
      if (files.length > 0) {
        event.preventDefault();
        processAndInsertImages(files);
      }
    },
    [processAndInsertImages, isMultimodalitySupportedForCurrentModel],
  );

  const {
    checkboxes,
    onToggleCheckbox,
    unCheckAll,
    setLineSelectionInteracted,
  } = useCheckboxes();

  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const handleManualStopStreamingClick = useCallback(() => {
    abort();
    void sendTelemetryEvent({
      scope: `stopStreaming`,
      success: true,
      error_message: "",
    });
  }, [abort, sendTelemetryEvent]);

  const [value, setValue, isSendImmediately, setIsSendImmediately] =
    useInputValue(() => unCheckAll());

  const onClearInformation = useCallback(
    () => dispatch(clearInformation()),
    [dispatch],
  );

  const { previewFiles, commands, requestCompletion } =
    useCommandCompletionAndPreviewFiles(
      checkboxes,
      attachedFiles.addFilesToInput,
    );

  const refs = useTourRefs();

  const fileInputRef = React.useRef<HTMLInputElement>(null);
  const [isProcessingFiles, setIsProcessingFiles] = React.useState(false);
  const [processedFiles, setProcessedFiles] = React.useState<ParsedFile[]>([]);

  const apiBase =
    config.uploadApiUrl ??
    config.lspUrl ??
    `http://127.0.0.1:${config.lspPort}`;

  const handleFileChange = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const selectedFiles = Array.from(event.target.files || []);
      if (selectedFiles.length === 0) return;

      setIsProcessingFiles(true);

      try {
        const { results, errors } = await parseFiles(selectedFiles, apiBase);

        if (results.length > 0) {
          setProcessedFiles((prev) => [...prev, ...results]);
        }
        if (errors.length > 0) {
          const names = errors
            .map((e: { file: string; error: string }) => `${e.file}: ${e.error}`)
            .join("; ");
          console.error("File processing errors:", names);
        }
      } catch (err) {
        console.error("File processing failed:", err);
      } finally {
        setIsProcessingFiles(false);
        if (event.target) event.target.value = "";
      }
    },
    [apiBase],
  );

  const removeProcessedFile = useCallback((filename: string) => {
    setProcessedFiles((prev) => prev.filter((f) => f.filename !== filename));
  }, []);

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    const hasProcessedFiles = processedFiles.length > 0;

    if (!disableSend && (trimmedValue.length > 0 || hasProcessedFiles)) {
      const valueWithFiles = attachedFiles.addFilesToInput(trimmedValue);
      const valueIncludingChecks = addCheckboxValuesToInput(
        valueWithFiles,
        checkboxes,
      );

      let messageToSubmit = valueIncludingChecks;
      if (hasProcessedFiles) {
        const fileContextBlocks = processedFiles
          .map(
            (r) =>
              `--- Content from ${r.filename} ---\n\n${r.text.trim()}\n\n`,
          )
          .join("");

        dispatch(setPendingProcessedContext(fileContextBlocks));

        const fileNames = processedFiles.map((f) => f.filename).join(", ");
        messageToSubmit =
          valueIncludingChecks + (fileNames ? ` (files: ${fileNames})` : "");
        dispatch(setLastUserMessageDisplayContent(messageToSubmit));
      } else {
        dispatch(setLastUserMessageDisplayContent(null));
      }

      setLineSelectionInteracted(false);
      onSubmit(messageToSubmit);
      setValue(() => "");
      unCheckAll();
      attachedFiles.removeAll();
      setProcessedFiles([]);
    }
  }, [
    value,
    disableSend,
    processedFiles,
    dispatch,
    attachedFiles,
    checkboxes,
    setLineSelectionInteracted,
    onSubmit,
    setValue,
    unCheckAll,
  ]);

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleHelpInfo = useCallback((info: React.ReactNode | null) => {
    setHelpInfo(info);
  }, []);

  const helpText = () => (
    <Flex direction="column">
      <Text size="2" weight="bold">
        Quick help for @-commands:
      </Text>
      <Text size="2">
        @definition &lt;class_or_function_name&gt; — find the definition and
        attach it.
      </Text>
      <Text size="2">
        @references &lt;class_or_function_name&gt; — find all references and
        attach them.
      </Text>
      <Text size="2">
        @file &lt;dir/filename.ext&gt; — attaches a single file to the chat.
      </Text>
      <Text size="2">@tree — workspace directory and files tree.</Text>
      <Text size="2">@web &lt;url&gt; — attach a webpage to the chat.</Text>
    </Flex>
  );

  const handleHelpCommand = useCallback(() => {
    setHelpInfo(helpText());
  }, []);

  const handleChange = useCallback(
    (command: string) => {
      setValue(command);
      const trimmedCommand = command.trim();
      if (!trimmedCommand) {
        setLineSelectionInteracted(false);
      } else {
        setLineSelectionInteracted(true);
      }

      if (trimmedCommand === "@help") {
        handleHelpInfo(helpText()); // This line has been fixed
      } else {
        handleHelpInfo(null);
      }
    },
    [handleHelpInfo, setValue, setLineSelectionInteracted],
  );

  const handleAgentIntegrationsClick = useCallback(() => {
    dispatch(push({ name: "integrations page" }));
    void sendTelemetryEvent({
      scope: `openIntegrations`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent]);

  useEffect(() => {
    if (isSendImmediately && !isWaiting && !isStreaming) {
      handleSubmit();
      setIsSendImmediately(false);
    }
  }, [
    isSendImmediately,
    isWaiting,
    isStreaming,
    handleSubmit,
    setIsSendImmediately,
  ]);

  if (globalError) {
    return (
      <ErrorCallout mt="2" onClick={onClearError} timeout={null}>
        {globalError}
      </ErrorCallout>
    );
  }

  if (information) {
    return (
      <InformationCallout mt="2" onClick={onClearInformation} timeout={2000}>
        {information}
      </InformationCallout>
    );
  }

  if (!isStreaming && pauseReasonsWithPause.pause) {
    return (
      <ToolConfirmation pauseReasons={pauseReasonsWithPause.pauseReasons} />
    );
  }

  return (
    <Card
      mt="1"
      className={styles.composerCard}
      style={{ paddingBottom: "1px", position: "relative" }}
    >
      {globalErrorType === "balance" && (
        <BallanceCallOut
          mt="0"
          mb="2"
          mx="0"
          onClick={() => dispatch(clearError())}
        />
      )}
      {shouldShowBalanceLow && <BallanceLowInformation mt="0" mb="2" mx="0" />}
      {!isOnline && (
        <Callout type="info" mb="2">
          Oops, seems that connection was lost... Check your internet connection
        </Callout>
      )}

      <Flex
        ref={(x) => refs.setChat(x)}
        style={{
          // TODO: direction can be done with prop `direction`
          flexDirection: "column",
          alignSelf: "stretch",
          flex: 1,
          width: "100%",
        }}
      >
        {helpInfo && (
          <Flex mb="3" direction="column">
            {helpInfo}
          </Flex>
        )}
        <Form
          disabled={disableSend}
          className={classNames(styles.chatForm__form, className)}
          onSubmit={handleSubmit}
        >
          <FilesPreview files={previewFiles} />

          <Flex className={styles.composerPromptRow} align="end" gap="1">
            <Flex direction="column" flexGrow="1" style={{ minWidth: 0 }}>
              <ComboBox
                onHelpClick={handleHelpCommand}
                commands={commands}
                requestCommandsCompletion={requestCompletion}
                value={value}
                onChange={handleChange}
                onSubmit={(event) => {
                  handleEnter(event);
                }}
                placeholder={
                  commands.completions.length < 1 ? "Type @ for commands" : ""
                }
                render={(props) => (
                  <TextArea
                    data-testid="chat-form-textarea"
                    required={true}
                    size="1"
                    // disabled={isStreaming}
                    {...props}
                    autoFocus={autoFocus}
                    style={{ boxShadow: "none", outline: "none" }}
                    onPaste={handlePastingFile}
                  />
                )}
              />
            </Flex>
            <div className={styles.composerPromptSend}>
              {(isWaiting || isStreaming) && !pauseReasonsWithPause.pause ? (
                <button
                  type="button"
                  title="Stop streaming"
                  aria-label="Stop streaming"
                  onClick={handleManualStopStreamingClick}
                  className={classNames(
                    craftifPanelBtn.root,
                    craftifPanelBtn.active,
                    styles.composerPromptSendButton,
                  )}
                >
                  <IoStop size={15} aria-hidden />
                </button>
              ) : (
                <button
                  type="submit"
                  disabled={disableSend}
                  title="Send message"
                  aria-label="Send message"
                  className={classNames(
                    craftifPanelBtn.root,
                    craftifPanelBtn.active,
                    styles.composerPromptSendButton,
                  )}
                >
                  <IoIosSend size={15} aria-hidden />
                </button>
              )}
            </div>
          </Flex>
          <Flex className={styles.composerToolbar} gap="1" wrap="wrap" py="1" px="2">
            {/* {isModelSelectVisible && <CapsSelect />} */}
            {/* <CapsSelect disabled={messages.length >= 1}/> */}

            <Flex className={styles.composerActions} justify="end" flexGrow="1" wrap="wrap" gap="2">
              <input
                type="file"
                multiple
                ref={fileInputRef}
                style={{ display: "none" }}
                onChange={handleFileChange}
              />
              {/* <ThinkingButton /> */}
              {/* {shouldAgentCapabilitiesBeShown && <AgentCapabilities inline />} */}
              <TokensPreview
                currentMessageQuery={attachedFiles.addFilesToInput(value)}
              />
              <Flex className={styles.composerIconRow} gap="2" align="center" justify="center">
                {/* <IconButton
                  size="1"
                  variant="ghost"
                  color={
                    lastSentCompression === "high"
                      ? "red"
                      : lastSentCompression === "medium"
                        ? "yellow"
                        : undefined
                  }
                  title="Compress chat and continue"
                  type="button"
                  onClick={() => void compressChat()}
                  disabled={
                    messages.length === 0 ||
                    isStreaming ||
                    isWaiting ||
                    unCalledTools
                  }
                  loading={compressChatRequest.isLoading || isCompressing}
                >
                  <ArchiveIcon />
                </IconButton> */}
                {/* {toolUse === "agent" && (
                  <AgentIntegrationsButton
                    title="Set up Agent Integrations"
                    size="1"
                    type="button"
                    onClick={handleAgentIntegrationsClick}
                    ref={(x) => refs.setSetupIntegrations(x)}
                  />
                )}
                {onClose && (
                  <BackToSideBarButton
                    disabled={isStreaming}
                    title="Return to sidebar"
                    size="1"
                    onClick={onClose}
                  />
                )} */}
                {config.features?.images !== false &&
                  isMultimodalitySupportedForCurrentModel && (
                    <AttachImagesButton />
                  )}
                <IconButton
                  size="1"
                  variant="ghost"
                  title="Attach Processed Documents (PDF/DOCX/PPTX/etc.)"
                  type="button"
                  onClick={() => fileInputRef.current?.click()}
                  disabled={isStreaming || isWaiting || isProcessingFiles}
                >
                  <FilePlusIcon />
                </IconButton>
                {/* TODO: Reserved space for microphone button coming later on */}
              </Flex>
            </Flex>
          </Flex>
        </Form>
        {processedFiles.length > 0 && (
          <Flex gap="2" wrap="wrap" mt="2" px="2" pb="2">
            {processedFiles.map((pf) => (
              <Flex
                key={pf.filename}
                align="center"
                gap="1"
                style={{
                  backgroundColor: "var(--accent-a3)",
                  padding: "2px 8px",
                  borderRadius: "12px",
                  fontSize: "12px",
                }}
              >
                <Text size="1">{pf.filename}</Text>
                <IconButton
                  size="1"
                  variant="ghost"
                  style={{ width: "16px", height: "16px", margin: 0, padding: 0 }}
                  onClick={() => removeProcessedFile(pf.filename)}
                >
                  <Cross2Icon />
                </IconButton>
              </Flex>
            ))}
          </Flex>
        )}
      </Flex>
      <FileList attachedFiles={attachedFiles} />

      <ChatControls
        // handle adding files
        host={config.host}
        checkboxes={checkboxes}
        onCheckedChange={onToggleCheckbox}
        attachedFiles={attachedFiles}
      />
    </Card>
  );
};
