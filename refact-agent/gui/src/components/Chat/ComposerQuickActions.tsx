import React, { useCallback } from "react";
import { FiWifi } from "react-icons/fi";
import { HiOutlineLightBulb } from "react-icons/hi2";
import { FaMicrochip } from "react-icons/fa6";
import { setInputValue } from "../ChatForm/actions";
import styles from "./ComposerQuickActions.module.css";

const POST_TARGET_ORIGIN =
  typeof window !== "undefined" &&
  window.location.origin &&
  window.location.origin !== "null"
    ? window.location.origin
    : "*";

/** Fills the chat composer via the same bridge as embedded hosts (`useInputValue`). */
export function injectTextIntoComposer(text: string): void {
  window.postMessage(
    setInputValue({ value: text, send_immediately: true }),
    POST_TARGET_ORIGIN,
  );
  // window.setTimeout(() => {
  //   requestAnimationFrame(() => {
  //     const el = document.querySelector<HTMLTextAreaElement>(
  //       '[data-testid="chat-form-textarea"]',
  //     );
  //     if (!el) return;
  //     el.focus();
  //     const len = el.value.length;
  //     el.setSelectionRange(len, len);
  //   });
  // }, 0);
}

const ACTIONS = [
  {
    id: "wifi",
    label: "Connect to Wi-Fi",
    text: "Connect to Wi-Fi",
    Icon: FiWifi,
    iconColor: "#38bdf8",
  },
  {
    id: "led",
    label: "Blink LED",
    text: "Blink onboard RGB LED in rainbow pattern",
    Icon: HiOutlineLightBulb,
    iconColor: "#f87171",
  },
  {
    id: "spi",
    label: "BLE Provisioning",
    text: "Connect to Wi-Fi over ble provisioning",
    Icon: FaMicrochip,
    iconColor: "#a78bfa",
  },
] as const;

export type ComposerQuickActionsProps = {
  disabled?: boolean;
};

export const ComposerQuickActions: React.FC<ComposerQuickActionsProps> = ({
  disabled = false,
}) => {
  const onPick = useCallback((text: string) => {
    if (disabled) return;
    injectTextIntoComposer(text);
  }, [disabled]);

  return (
    <div
      className={styles.row}
      data-testid="composer-quick-actions"
      role="group"
      aria-label="Quick actions"
    >
      {ACTIONS.map(({ id, label, text, Icon, iconColor }) => (
        <button
          key={id}
          type="button"
          className={styles.chip}
          disabled={disabled}
          aria-label={`Run: ${label}`}
          onClick={() => onPick(text)}
        >
          <span className={styles.iconWrap} aria-hidden>
            <Icon size={16} style={{ color: iconColor }} />
          </span>
          <span className={styles.label}>{label}</span>
        </button>
      ))}
    </div>
  );
};
