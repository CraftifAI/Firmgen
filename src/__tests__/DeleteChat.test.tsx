import { render } from "../utils/test-utils";
import { describe, expect, it } from "vitest";
import {
  server,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
  goodCaps,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";
import { HistoryState } from "../features/History/historySlice";

describe("Delete a Chat form history", () => {
  server.use(
    goodUser,
    goodPing,
    chatLinks,
    telemetryChat,
    telemetryNetwork,
    goodCaps,
  );
  it("can delete a chat", async () => {
    const now = new Date().toISOString();
    const history: HistoryState = {
      abc123: {
        title: "Test title",
        isTitleGenerated: false,
        messages: [],
        id: "abc123",
        model: "foo",
        tool_use: "quick",
        new_chat_suggested: {
          wasSuggested: false,
        },
        createdAt: now,
        updatedAt: now,
        read: true,
      },
    };
    const { user, store, ...app } = render(<InnerApp />, {
      preloadedState: {
        history,
        teams: {
          group: { id: "123", name: "test" },
          workspace: { ws_id: "123", root_group_name: "test" },
          skipped: false,
        },
        pages: [{ name: "chat" }],
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

    const itemTitleToDelete = "Test title";

    await app.findByText(itemTitleToDelete);

    const optionsTrigger = app.getByRole("button", { name: "Chat options" });
    await user.click(optionsTrigger);

    const deleteItem = await app.findByRole("menuitem", { name: "Delete chat" });
    await user.click(deleteItem);

    expect(store.getState().history).toEqual({});
  });
});
