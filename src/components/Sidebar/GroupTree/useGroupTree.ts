import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FlexusTreeNode } from "./GroupTree";
import {
  CreateGroupDocument,
  CreateGroupMutation,
  CreateGroupMutationVariables,
  NavTreeSubsDocument,
  NavTreeSubsSubscription,
  NavTreeWantWorkspacesDocument,
  NavTreeWantWorkspacesQuery,
  NavTreeWantWorkspacesQueryVariables,
} from "../../../../generated/documents";
import { useMutation, useQuery } from "urql";
import {
  cleanupInsertedLater,
  markForDelete,
  pruneNodes,
  updateTree,
} from "./utils";
import { useSmartSubscription } from "../../../hooks/useSmartSubscription";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
  useOpenUrl,
  useResizeObserver,
} from "../../../hooks";
import { isDetailMessage, teamsApi } from "../../../services/refact";
import { NodeApi } from "react-arborist";
import {
  resetActiveGroup,
  resetActiveWorkspace,
  selectActiveWorkspace,
  setActiveGroup,
  setActiveWorkspace,
  setSkippedWorkspaceSelection,
} from "../../../features/Teams";
import { setError, clearError } from "../../../features/Errors/errorsSlice";
import { selectConfig, selectEmbedded } from "../../../features/Config/configSlice";
import { newChatAction } from "../../../events";
import { popBackTo, push } from "../../../features/Pages/pagesSlice";
import { clearPauseReasonsAndHandleToolsStatus } from "../../../features/ToolConfirmation/confirmationSlice";

export function useGroupTree() {
  const [groupTreeData, setGroupTreeData] = useState<FlexusTreeNode[]>([]);
  const [createFolderChecked, setCreateFolderChecked] = useState(false);

  const currentTeamsWorkspace = useAppSelector(selectActiveWorkspace);
  const isEmbedded = useAppSelector(selectEmbedded);
  const openUrl = useOpenUrl();

  const [_, createGroup] = useMutation<
    CreateGroupMutation,
    CreateGroupMutationVariables
  >(CreateGroupDocument);

  const [teamsWorkspaces] = useQuery<
    NavTreeWantWorkspacesQuery,
    NavTreeWantWorkspacesQueryVariables
  >({
    query: NavTreeWantWorkspacesDocument,
  });

  const filterNodesByNodeType = useCallback(
    (nodes: FlexusTreeNode[], type: string): FlexusTreeNode[] => {
      return nodes
        .filter((node) => node.treenodeType === type)
        .map((node) => {
          const children =
            node.treenodeChildren.length > 0
              ? filterNodesByNodeType(node.treenodeChildren, type)
              : [];
          return {
            ...node,
            treenodeChildren: children,
          };
        });
    },
    [],
  );

  const filteredGroupTreeData = useMemo(() => {
    return filterNodesByNodeType(groupTreeData, "group");
  }, [groupTreeData, filterNodesByNodeType]);

  const touchNode = useCallback(
    (path: string, title: string, type: string, id: string) => {
      if (!path) return;
      setGroupTreeData((prevTree) => {
        const parts = path.split("/");
        return updateTree(prevTree, parts, "", id, path, title, type);
      });
    },
    [setGroupTreeData],
  );

  const handleEveryTreeUpdate = useCallback(
    (data: NavTreeSubsSubscription | undefined) => {
      const u = data?.tree_subscription;
      if (!u) return;
      switch (u.treeupd_action) {
        case "TREE_REBUILD_START":
          setGroupTreeData((prev) => markForDelete(prev));
          break;
        case "TREE_UPDATE":
          touchNode(
            u.treeupd_path,
            u.treeupd_title,
            u.treeupd_type,
            u.treeupd_id,
          );
          break;
        case "TREE_REBUILD_FINISHED":
          setTimeout(() => {
            setGroupTreeData((prev) => pruneNodes(prev));
          }, 500);
          setTimeout(() => {
            setGroupTreeData((prev) => cleanupInsertedLater(prev));
          }, 3000);
          break;
        default:
          // eslint-disable-next-line no-console
          console.warn("TREE SUBS:", u.treeupd_action);
      }
    },
    [touchNode],
  );

  useSmartSubscription<NavTreeSubsSubscription, { ws_id: string }>({
    query: NavTreeSubsDocument,
    variables: {
      ws_id: currentTeamsWorkspace?.ws_id ?? "",
    },
    skip: currentTeamsWorkspace === null,
    onUpdate: handleEveryTreeUpdate,
  });

  const dispatch = useAppDispatch();
  const { setActiveTeamsGroupInIDE, setActiveTeamsWorkspaceInIDE } =
    useEventsBusForIDE();

  const [setActiveGroupIdTrigger] = teamsApi.useSetActiveGroupIdMutation();
  const [currentSelectedTeamsGroupNode, setCurrentSelectedTeamsGroupNode] =
    useState<FlexusTreeNode | null>(null);

  const treeParentRef = useRef<HTMLDivElement | null>(null);
  const [treeHeight, setTreeHeight] = useState<number>(
    treeParentRef.current?.clientHeight ?? 0,
  );

  const calculateAndSetSpace = useCallback(() => {
    if (!treeParentRef.current) {
      return;
    }
    setTreeHeight(treeParentRef.current.clientHeight);
    // NOTE: this is a hack to avoid the tree being with 0 width/height even when data appears
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [treeParentRef, filteredGroupTreeData]);

  useResizeObserver(treeParentRef.current, calculateAndSetSpace);

  useEffect(() => {
    calculateAndSetSpace();
  }, [calculateAndSetSpace]);

  const onGroupSelect = useCallback((nodes: NodeApi<FlexusTreeNode>[]) => {
    if (nodes.length === 0) return;
    const groupNode = nodes[0].data;
    setCurrentSelectedTeamsGroupNode(groupNode);
  }, []);

  const onGroupSelectionConfirm = useCallback(
    async (group: FlexusTreeNode): Promise<boolean> => {
      const newGroup = {
        id: group.treenodeId,
        name: group.treenodeTitle,
      };

      setActiveTeamsGroupInIDE(newGroup);
      
      // In embedded mode, just update Redux state - no need to call agent API
      // active_group_id is only needed for cloud knowledge features, not local VecDB
      // VecDB indexing uses workspace_folders (set via /v1/lsp-initialize)
      if (isEmbedded) {
        dispatch(setActiveGroup(newGroup));
        return true;
      }
      
      // Only call API in cloud mode (for cloud knowledge features)
      try {
        const result = await setActiveGroupIdTrigger({
          group_id: group.treenodeId,
        });
        if (result.data) {
          dispatch(setActiveGroup(newGroup));
          return true;
        } else {
          // TODO: rework error handling
          let errorMessage: string;
          if ("data" in result.error && isDetailMessage(result.error.data)) {
            errorMessage = result.error.data.detail;
          } else {
            errorMessage =
              "Error: Something went wrong while selecting a group. Try again.";
          }
          dispatch(setError(errorMessage));
          return false;
        }
      } catch {
        dispatch(resetActiveGroup());
        return false;
      }
    },
    [dispatch, setActiveGroupIdTrigger, setActiveTeamsGroupInIDE, isEmbedded],
  );

  const navigateToNewChat = useCallback(() => {
    dispatch(newChatAction());
    dispatch(
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
    );
    dispatch(popBackTo({ name: "history" }));
    dispatch(push({ name: "chat" }));
  }, [dispatch]);

  const onWorkspaceSelectChange = useCallback(
    (value: string) => {
      const maybeWorkspace =
        teamsWorkspaces.data?.query_basic_stuff.workspaces.find(
          (w) => w.ws_id === value,
        );
      if (maybeWorkspace) {
        setActiveTeamsWorkspaceInIDE(maybeWorkspace);
        dispatch(setActiveWorkspace(maybeWorkspace));
        setCurrentSelectedTeamsGroupNode(null);
      }
    },
    [
      dispatch,
      setActiveTeamsWorkspaceInIDE,
      teamsWorkspaces.data?.query_basic_stuff.workspaces,
    ],
  );

  const handleCreateWorkspaceClick = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      event.stopPropagation();
      openUrl("http://app.refact.ai/profile?action=create-workspace");
    },
    [openUrl],
  );

  const currentWorkspaceName =
    useAppSelector(selectConfig).currentWorkspaceName ?? "New Project";

  const isMatchingGroupNameWithWorkspace = useMemo(() => {
    return (
      currentSelectedTeamsGroupNode?.treenodeTitle === currentWorkspaceName
    );
  }, [currentSelectedTeamsGroupNode?.treenodeTitle, currentWorkspaceName]);

  const handleConfirmSelectionClick = useCallback(async () => {
    // In embedded mode, if no group is selected, use the default group from workspace
    let groupToConfirm = currentSelectedTeamsGroupNode;
    if (!groupToConfirm && isEmbedded && currentTeamsWorkspace) {
      groupToConfirm = {
        treenodeId: currentTeamsWorkspace.ws_root_group_id,
        treenodeTitle: currentTeamsWorkspace.root_group_name,
        treenodeType: "group",
        treenodePath: `/group:${currentTeamsWorkspace.ws_root_group_id}`,
        treenode__DeleteMe: false,
        treenode__InsertedLater: false,
        treenodeChildren: [],
        treenodeExpanded: false,
      };
    }
    
    if (!groupToConfirm) return;
    
    if (createFolderChecked && !isMatchingGroupNameWithWorkspace) {
      const result = await createGroup({
        fgroup_name: currentWorkspaceName,
        fgroup_parent_id: groupToConfirm.treenodeId,
      });

      if (result.error) {
        dispatch(setError(result.error.message));
        return;
      }

      const newGroup = result.data?.group_create;
      if (newGroup) {
        const newNode: FlexusTreeNode = {
          treenodeId: newGroup.fgroup_id,
          treenodeTitle: newGroup.fgroup_name,
          treenodeType: "group",
          treenodePath: `${groupToConfirm.treenodePath}/group:${newGroup.fgroup_id}`,
          treenode__DeleteMe: false,
          treenode__InsertedLater: false,
          treenodeChildren: [],
          treenodeExpanded: false,
        };
        setCurrentSelectedTeamsGroupNode(newNode);
        const ok = await onGroupSelectionConfirm(newNode);
        if (ok) navigateToNewChat();
      }
    } else {
      const ok = await onGroupSelectionConfirm(groupToConfirm);
      if (ok) {
        setCurrentSelectedTeamsGroupNode(null);
        navigateToNewChat();
      }
    }
  }, [
    dispatch,
    createGroup,
    currentSelectedTeamsGroupNode,
    setCurrentSelectedTeamsGroupNode,
    onGroupSelectionConfirm,
    navigateToNewChat,
    currentWorkspaceName,
    createFolderChecked,
    isMatchingGroupNameWithWorkspace,
    isEmbedded,
    currentTeamsWorkspace,
  ]);

  const handleSkipWorkspaceSelection = useCallback(() => {
    dispatch(setSkippedWorkspaceSelection(true));
    dispatch(resetActiveWorkspace());
    navigateToNewChat();
  }, [dispatch, navigateToNewChat]);
  
  // Default workspace for embedded mode
  const defaultEmbeddedWorkspace = useMemo(() => {
    if (!isEmbedded) return null;
    return {
      ws_id: "local-workspace",
      ws_owner_fuser_id: "local-user",
      ws_root_group_id: "local-root-group",
      root_group_name: "Local Workspace",
      have_coins_exactly: 0,
      have_coins_enough: false,
      have_admin: true,
    };
  }, [isEmbedded]);
  
  const availableWorkspaces = useMemo(() => {
    // If we have data from GraphQL, use it
    if (teamsWorkspaces.data?.query_basic_stuff.workspaces) {
      return teamsWorkspaces.data.query_basic_stuff.workspaces;
    }
    // In embedded mode, always show the default workspace
    // This ensures the workspace selector is always available in embedded mode
    if (isEmbedded && defaultEmbeddedWorkspace) {
      // Use current workspace if set, otherwise use default
      return currentTeamsWorkspace ? [currentTeamsWorkspace] : [defaultEmbeddedWorkspace];
    }
    return [];
  }, [teamsWorkspaces.data?.query_basic_stuff.workspaces, isEmbedded, currentTeamsWorkspace, defaultEmbeddedWorkspace]);

  useEffect(() => {
    if (availableWorkspaces.length === 1) {
      dispatch(setActiveWorkspace(availableWorkspaces[0]));
      setActiveTeamsWorkspaceInIDE(availableWorkspaces[0]);
    }
  }, [dispatch, setActiveTeamsWorkspaceInIDE, availableWorkspaces]);
  
  // In embedded mode, ensure default workspace is set if not already set
  useEffect(() => {
    if (isEmbedded && defaultEmbeddedWorkspace && !currentTeamsWorkspace) {
      dispatch(setActiveWorkspace(defaultEmbeddedWorkspace));
      setActiveTeamsWorkspaceInIDE(defaultEmbeddedWorkspace);
      
      // In embedded mode, automatically select the root group so Confirm button is enabled
      const defaultGroupNode: FlexusTreeNode = {
        treenodeId: defaultEmbeddedWorkspace.ws_root_group_id,
        treenodeTitle: defaultEmbeddedWorkspace.root_group_name,
        treenodeType: "group",
        treenodePath: `/group:${defaultEmbeddedWorkspace.ws_root_group_id}`,
        treenode__DeleteMe: false,
        treenode__InsertedLater: false,
        treenodeChildren: [],
        treenodeExpanded: false,
      };
      setCurrentSelectedTeamsGroupNode(defaultGroupNode);
    }
  }, [isEmbedded, defaultEmbeddedWorkspace, currentTeamsWorkspace, dispatch, setActiveTeamsWorkspaceInIDE, setCurrentSelectedTeamsGroupNode]);

  const config = useAppSelector(selectConfig);
  
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
  
  // Handler to set workspace folder on the agent
  const handleSetWorkspaceFolder = useCallback(async (folderPath: string) => {
    if (!isEmbedded) return;
    
    try {
      const port = config.lspPort || 8001;
      const url = `http://127.0.0.1:${port}/v1/lsp-initialize`;
      
      // Convert folder path to file:// URL format
      // The agent expects file:// URLs with proper encoding
      let folderUrl: string;
      if (folderPath.startsWith("file://")) {
        folderUrl = folderPath;
      } else {
        // Ensure path starts with / and convert to file:// URL
        const normalizedPath = folderPath.startsWith("/") ? folderPath : `/${folderPath}`;
        folderUrl = `file://${normalizedPath}`;
      }
      
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(config.apiKey ? { Authorization: `Bearer ${config.apiKey}` } : {}),
        },
        body: JSON.stringify({
          project_roots: [folderUrl],
        }),
      });
      
      if (!response.ok) {
        throw new Error(`Failed to set workspace folder: ${response.statusText}`);
      }
      
      const result = await response.json();
      console.log("Workspace folder set successfully:", result);
      
      // Clear any previous errors
      dispatch(clearError());
    } catch (error) {
      console.error("Error setting workspace folder:", error);
      const errorMessage = error instanceof Error ? error.message : String(error);
      if (errorMessage) {
        dispatch(setError(`Failed to set workspace folder: ${errorMessage}`));
      }
    }
  }, [isEmbedded, config, dispatch]);
  

  return {
    // Refs
    treeParentRef,
    // Data
    groupTreeData,
    filteredGroupTreeData,
    teamsWorkspaces,
    availableWorkspaces,
    // Current states
    currentTeamsWorkspace,
    currentSelectedTeamsGroupNode,
    createFolderChecked,
    // Dimensions
    treeHeight,
    // Actions
    onGroupSelect,
    handleCreateNewChat,
    onGroupSelectionConfirm,
    onWorkspaceSelectChange,
    touchNode,
    handleSkipWorkspaceSelection,
    handleConfirmSelectionClick,
    handleCreateWorkspaceClick,
    handleSetWorkspaceFolder,
    isEmbedded,
    // Setters
    setGroupTreeData,
    setCurrentSelectedTeamsGroupNode,
    setCreateFolderChecked,
  };
}
