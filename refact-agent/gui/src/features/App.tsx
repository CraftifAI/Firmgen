import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";
import { Flex } from "@radix-ui/themes";
import { Chat, newChatAction, selectChatId, selectIsStreaming } from "./Chat";
import { Sidebar } from "../components/Sidebar/Sidebar";
import {
  useAppSelector,
  useAppDispatch,
  useConfig,
  useEffectOnce,
  useEventsBusForIDE,
} from "../hooks";
import { FIMDebug } from "./FIM";
import { store, persistor, RootState } from "../app/store";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { Theme } from "../components/Theme";
import { useEventBusForWeb } from "../hooks/useEventBusForWeb";
import { Statistics } from "./Statistics";
import { ContextPayloadSidebar } from "../components/ContextPayloadSidebar";
import {
  push,
  popBackTo,
  pop,
  change,
  selectPages,
} from "../features/Pages/pagesSlice";
import { TourProvider } from "./Tour";
import { Tour } from "../components/Tour";
import { TourEnd } from "../components/Tour/TourEnd";
import { useEventBusForApp } from "../hooks/useEventBusForApp";
import { AbortControllerProvider } from "../contexts/AbortControllers";
import { Toolbar } from "../components/Toolbar";
import { Tab } from "../components/Toolbar/Toolbar";
import { PageWrapper } from "../components/PageWrapper";
import { ThreadHistory } from "./ThreadHistory";
import { Integrations } from "./Integrations";
import { Providers } from "./Providers";
import { UserSurvey } from "./UserSurvey";
import { integrationsApi } from "../services/refact";
import { LoginPage } from "./Login";
import { AdminUsagePage } from "./AdminPanel/AdminUsagePage";
import { ProjectSourcesView } from "./ProjectSources/ProjectSourcesView";
import { sessionJwt } from "../hooks/useCraftifAuth";

import styles from "./App.module.css";
import classNames from "classnames";
import { usePatchesAndDiffsEventsForIDE } from "../hooks/usePatchesAndDiffEventsForIDE";
import { UrqlProvider } from "../../urqlProvider";
import { selectActiveGroup } from "./Teams";
import { useActiveTeamsGroup } from "../hooks/useActiveTeamsGroup";

export interface AppProps {
  style?: React.CSSProperties;
}

export const InnerApp: React.FC<AppProps> = ({ style }: AppProps) => {
  const dispatch = useAppDispatch();

  const pages = useAppSelector(selectPages);
  const isStreaming = useAppSelector(selectIsStreaming);

  const isPageInHistory = useCallback(
    (pageName: string) => {
      return pages.some((page) => page.name === pageName);
    },
    [pages],
  );

  const { chatPageChange, setIsChatStreaming, setIsChatReady, setupHost } =
    useEventsBusForIDE();
  const tourState = useAppSelector((state: RootState) => state.tour);
  const historyState = useAppSelector((state: RootState) => state.history);
  const maybeCurrentActiveGroup = useAppSelector(selectActiveGroup);
  const chatId = useAppSelector(selectChatId);
  const { groupSelectionEnabled } = useActiveTeamsGroup();
  useEventBusForWeb();
  useEventBusForApp();
  usePatchesAndDiffsEventsForIDE();

  const [isPaddingApplied, setIsPaddingApplied] = useState<boolean>(false);

  const handlePaddingShift = (state: boolean) => {
    setIsPaddingApplied(state);
  };

  const config = useConfig();

  const isLoggedIn =
    isPageInHistory("history") ||
    isPageInHistory("chat") ||
    isPageInHistory("project sources");

  useEffect(() => {
    if (!sessionJwt) return;
    if (config.apiKey === sessionJwt && config.addressURL) return;

    setupHost({
      type: "enterprise",
      apiKey: sessionJwt,
      endpointAddress: "http://127.0.0.1:8002",
    });
  }, [config.apiKey, config.addressURL, setupHost]);

  useEffect(() => {
    if (!sessionJwt) {
      if (pages[pages.length - 1]?.name !== "login page") {
        dispatch(popBackTo({ name: "login page" }));
      }
      return;
    }

    if (config.apiKey && config.addressURL && !isLoggedIn) {
      if (
        Object.keys(historyState).length === 0 &&
        // TODO: rework when better router will be implemented
        maybeCurrentActiveGroup
      ) {
        dispatch(push({ name: "history" }));
        dispatch(newChatAction());
        dispatch(push({ name: "chat" }));
      } else {
        dispatch(push({ name: "history" }));
      }
    }
    if (!config.apiKey && !config.addressURL && isLoggedIn) {
      dispatch(popBackTo({ name: "login page" }));
    }
  }, [
    config.apiKey,
    config.addressURL,
    isLoggedIn,
    dispatch,
    tourState,
    historyState,
    maybeCurrentActiveGroup,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    sessionJwt,
    pages[pages.length - 1]?.name,
  ]);

  useEffect(() => {
    if (pages.length > 1) {
      const currentPage = pages.slice(-1)[0];
      chatPageChange(currentPage.name);
    }
  }, [pages, chatPageChange]);

  useEffect(() => {
    setIsChatStreaming(isStreaming);
  }, [isStreaming, setIsChatStreaming]);

  useEffectOnce(() => {
    setIsChatReady(true);
  });

  // Skip standalone history page when workspace/group tree is not required (chat sidebar lists threads).
  useLayoutEffect(() => {
    if (groupSelectionEnabled) return;
    const top = pages[pages.length - 1];
    if (top?.name !== "history") return;
    const hasChatPage = pages.some((p) => p.name === "chat");
    if (hasChatPage) {
      dispatch(popBackTo({ name: "chat" }));
    } else {
      dispatch(change({ name: "chat" }));
    }
  }, [pages, groupSelectionEnabled, dispatch]);

  const goBack = () => {
    dispatch(pop());
  };

  const goBackFromIntegrations = () => {
    dispatch(pop());
    dispatch(integrationsApi.util.resetApiState());
  };

  const page = pages[pages.length - 1];

  const activeTab: Tab | undefined = useMemo(() => {
    if (page.name === "chat") {
      return {
        type: "chat",
        id: chatId,
      };
    }
    if (page.name === "history") {
      return {
        type: "dashboard",
      };
    }
    if (page.name === "project sources") {
      return {
        type: "dashboard",
      };
    }
  }, [page, chatId]);

  return (
    <Flex
      align="stretch"
      direction="column"
      style={style}
      className={classNames(styles.rootFlex, {
        [styles.integrationsPagePadding]:
          page.name === "integrations page" && isPaddingApplied,
      })}
    >
      <PageWrapper
        host={config.host}
        style={{
          paddingRight: page.name === "integrations page" ? 0 : undefined,
        }}
      >
        <UserSurvey />
        {page.name === "login page" && <LoginPage />}
        {activeTab && <Toolbar activeTab={activeTab} />}
        {page.name === "tour end" && <TourEnd />}
        {page.name === "history" && groupSelectionEnabled && (
          <Sidebar
            takingNotes={false}
            onOpenChatInTab={undefined}
            style={{
              alignSelf: "stretch",
              height: "calc(100% - var(--space-5)* 2)",
            }}
          />
        )}
        {page.name === "chat" && (
          <Chat
            host={config.host}
            tabbed={config.tabbed}
            backFromChat={goBack}
          />
        )}
        {page.name === "project sources" && (
          <ProjectSourcesView projectId={page.projectId} />
        )}
        {page.name === "fill in the middle debug page" && (
          <FIMDebug host={config.host} tabbed={config.tabbed} />
        )}
        {page.name === "statistics page" && (
          <Statistics
            backFromStatistic={goBack}
            tabbed={config.tabbed}
            host={config.host}
            onCloseStatistic={goBack}
          />
        )}
        {page.name === "context payload page" && (
          <Flex
            align="stretch"
            justify="center"
            style={{ width: "100%", flex: 1, minHeight: 0 }}
          >
            <ContextPayloadSidebar
              variant="page"
              onBackToChat={() => {
                const hasChatPage = pages.some((p) => p.name === "chat");
                if (hasChatPage) {
                  dispatch(popBackTo({ name: "chat" }));
                  return;
                }
                dispatch(popBackTo({ name: "history" }));
                dispatch(push({ name: "chat" }));
              }}
            />
          </Flex>
        )} 
        {page.name === "integrations page" && (
          <Integrations
            backFromIntegrations={goBackFromIntegrations}
            tabbed={config.tabbed}
            host={config.host}
            onCloseIntegrations={goBackFromIntegrations}
            handlePaddingShift={handlePaddingShift}
          />
        )}
        {page.name === "providers page" && (
          <Providers
            backFromProviders={goBack}
            tabbed={config.tabbed}
            host={config.host}
          />
        )}
        {page.name === "thread history page" && (
          <ThreadHistory
            backFromThreadHistory={goBack}
            tabbed={config.tabbed}
            host={config.host}
            onCloseThreadHistory={goBack}
            chatId={page.chatId}
          />
        )}
        {page.name === "admin usage page" && <AdminUsagePage />}
      </PageWrapper>
      <Tour page={pages[pages.length - 1].name} />
    </Flex>
  );
};

// TODO: move this to the `app` directory.
export const App = () => {
  return (
    <Provider store={store}>
      <UrqlProvider>
        <PersistGate persistor={persistor}>
          <Theme>
            <TourProvider>
              <AbortControllerProvider>
                <InnerApp />
              </AbortControllerProvider>
            </TourProvider>
          </Theme>
        </PersistGate>
      </UrqlProvider>
    </Provider>
  );
};
