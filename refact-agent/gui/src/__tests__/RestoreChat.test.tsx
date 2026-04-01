import { render } from "../utils/test-utils";
import { describe, expect, test } from "vitest";
import {
  server,
  goodPrompts,
  goodCaps,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";

describe("Restore Chat from history", () => {
  test("Restore chat from history", async () => {
    server.use(
      goodPing,
      goodCaps,
      goodPrompts,
      noTools,
      noCommandPreview,
      noCompletions,
      goodUser,
      chatLinks,
      telemetryChat,
      telemetryNetwork,
    );

    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "login page" }, { name: "chat" }],
        teams: {
          group: { id: "123", name: "test" },
          workspace: { ws_id: "123", root_group_name: "test" },
          skipped: false,
        },
        history: {
          id: {
            title: "test title",
            isTitleGenerated: true,
            id: "id",
            createdAt: "0",
            updatedAt: "0",
            model: "test",
            tool_use: "explore",
            messages: [
              { role: "user", content: "test user message", checkpoints: [] },
              { role: "assistant", content: "👋" },
            ],
            new_chat_suggested: {
              wasSuggested: false,
            },
            read: true,
          },
        },
        config: {
          apiKey: "test",
          lspPort: 8001,
          themeProps: {},
          host: "vscode",
          addressURL: "Refact",
          features: {
            statistics: true,
            vecdb: true,
            ast: true,
            images: true,
            embedded: true,
          },
        },
      },
    });

    const btn = app.getByText("test title");
    await user.click(btn);

    expect(app.queryByText("test user message")).not.toBeNull();

    expect(app.queryByText("👋")).not.toBeNull();
  });
});
