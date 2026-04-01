import { useEffect } from "react";
import { useLocalStorage } from "usehooks-ts";
import { isLogOut, isOpenExternalUrl, isSetupHost } from "../events/setup";
import { useAppDispatch } from "./useAppDispatch";
import { useConfig } from "./useConfig";
import { updateConfig } from "../features/Config/configSlice";

// all of the events that are normally handeled by the IDE
// are handled here for the web version.
export function useEventBusForWeb() {
  const config = useConfig();
  const [addressURL, setAddressURL] = useLocalStorage("lspUrl", "");
  const [apiKey, setApiKey] = useLocalStorage("apiKey", "");
  const dispatch = useAppDispatch();

  useEffect(() => {
    if (config.host !== "web") {
      return;
    }

    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (isOpenExternalUrl(event.data)) {
        const { url } = event.data.payload;
        window.open(url, "_blank")?.focus();
      }

      if (isSetupHost(event.data)) {
        const { host } = event.data.payload;
        let nextAddress = "";
        let nextKey = "";
        if (host.type === "cloud") {
          nextAddress = "Refact";
          nextKey = host.apiKey;
        } else if (host.type === "self") {
          nextAddress = host.endpointAddress;
          nextKey = "any-will-work-for-local-server";
        } else {
          nextAddress = host.endpointAddress;
          nextKey = host.apiKey;
        }
        setAddressURL(nextAddress);
        setApiKey(nextKey);
        const lspPort = Number(__REFACT_LSP_PORT__) || 8486;
        dispatch(
          updateConfig({
            addressURL: nextAddress,
            apiKey: nextKey,
            lspPort,
          }),
        );
      }

      if (isLogOut(event.data)) {
        setAddressURL("");
        setApiKey("");
        dispatch(updateConfig({ addressURL, apiKey }));
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [setApiKey, setAddressURL, config.host, dispatch, addressURL, apiKey]);

  useEffect(() => {
    if (config.host !== "web") {
      return;
    }
    dispatch(updateConfig({ addressURL, apiKey }));
  }, [apiKey, addressURL, dispatch, config.host]);
}
