import { useEffect, useRef, useState } from "react";

export type UsePirChatAnchorOptions = {
  chatId: string | null;
  projectPath: string | null;
  /** True once main/app_config.h exists for the codegen turn. */
  codegenReady: boolean;
  /** Assistant turn id when codegen completed. */
  agentTurnId: string | null;
  agentIsWorking: boolean;
};

/**
 * Pins the inline PIR block to the assistant turn that produced firmware code.
 * Once anchored, the block stays in chat history (does not follow new messages).
 */
export function usePirChatAnchor({
  chatId,
  projectPath,
  codegenReady,
  agentTurnId,
  agentIsWorking,
}: UsePirChatAnchorOptions): {
  anchorTurnId: string | null;
  effectiveProjectPath: string;
  showBlock: boolean;
} {
  const [anchorTurnId, setAnchorTurnId] = useState<string | null>(null);
  const [latchedProjectPath, setLatchedProjectPath] = useState("");
  const loggedAnchorRef = useRef<string | null>(null);

  useEffect(() => {
    setAnchorTurnId(null);
    setLatchedProjectPath("");
    loggedAnchorRef.current = null;
  }, [chatId]);

  useEffect(() => {
    const path = projectPath?.trim() ?? "";
    if (path) setLatchedProjectPath(path);
  }, [projectPath]);

  useEffect(() => {
    if (anchorTurnId != null && anchorTurnId !== "") return;
    if (!codegenReady || !agentTurnId || agentIsWorking) return;
    setAnchorTurnId(agentTurnId);
  }, [anchorTurnId, codegenReady, agentTurnId, agentIsWorking]);

  useEffect(() => {
    if (!anchorTurnId || loggedAnchorRef.current === anchorTurnId) return;
    loggedAnchorRef.current = anchorTurnId;
  }, [anchorTurnId, chatId, latchedProjectPath, projectPath]);

  const trimmedPath = projectPath?.trim();
  const effectiveProjectPath =
    trimmedPath && trimmedPath.length > 0 ? trimmedPath : latchedProjectPath;
  const showBlock = Boolean(chatId && effectiveProjectPath && anchorTurnId);

  return { anchorTurnId, effectiveProjectPath, showBlock };
}
