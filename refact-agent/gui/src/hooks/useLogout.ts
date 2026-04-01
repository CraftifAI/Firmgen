import { useCallback } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { usePostMessage } from "./usePostMessage";
import { EVENT_NAMES_FROM_SETUP } from "../events/setup";
import { setAddressURL, setApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";
import { clearStoredCraftifDisplayName } from "./useCraftifAuth";

export const useLogout = () => {
  const postMessage = usePostMessage();
  const dispatch = useAppDispatch();
  const [removeUser, _] = smallCloudApi.useRemoveUserFromCacheMutation();

  const logout = useCallback(() => {
    clearStoredCraftifDisplayName();
    postMessage({ type: EVENT_NAMES_FROM_SETUP.LOG_OUT });
    dispatch(setApiKey(null));
    dispatch(setAddressURL(""));
    removeUser(undefined).catch(() => ({}));
  }, [dispatch, postMessage, removeUser]);

  return logout;
};
