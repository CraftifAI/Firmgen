import React from "react";
import {
  Box,
  Button,
  Container,
  Flex,
  Heading,
  Text,
  TextField,
} from "@radix-ui/themes";
import { useAppDispatch } from "../../hooks";
import { updateConfig } from "../Config/configSlice";

type LauncherStartResponse = {
  ok: boolean;
  status: string;
  lspUrl?: string;
  httpPort?: number;
  error?: string;
};

const DEFAULTS = {
  launcherUrl: "http://127.0.0.1:8009",
  addressUrl: "http://127.0.0.1:8002",
  apiKey: "embedded-local",
  binaryPath: "/absolute/path/to/refact-lsp",
  vecDbPath: "/absolute/path/to/esp32_s3_32n8r.vecdb",
  espIdfPath: "/absolute/path/to/esp-idf",
  boardDefinition: "esp32-s3-devkitc-1-n32r8v",
  platform: "esp32",
  workspacePath: "/absolute/path/to/workspace",
  httpPort: "8486",
};

export const EmbeddedBootstrapPage: React.FC = () => {
  const dispatch = useAppDispatch();

  const [form, setForm] = React.useState(DEFAULTS);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const onChange =
    (key: keyof typeof DEFAULTS) =>
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setForm((prev) => ({ ...prev, [key]: e.target.value }));
    };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${form.launcherUrl}/v1/launcher/start`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          binaryPath: form.binaryPath,
          addressUrl: form.addressUrl,
          apiKey: form.apiKey,
          vecDbPath: form.vecDbPath,
          espIdfPath: form.espIdfPath,
          boardDefinition: form.boardDefinition,
          platform: form.platform,
          workspacePath: form.workspacePath,
          httpPort: Number(form.httpPort),
        }),
      });

      const result = (await response.json()) as LauncherStartResponse;

      if (!response.ok || !result.ok || !result.lspUrl) {
        throw new Error(result.error || "Failed to start launcher");
      }

      const lspPort = result.httpPort ?? Number(form.httpPort);

      // Persist for convenience
      const { apiKey, ...safeToStore } = form;
      localStorage.setItem("embedded-bootstrap-config", JSON.stringify(safeToStore));

      dispatch(
        updateConfig({
          lspUrl: result.lspUrl,
          lspPort,
          addressURL: result.lspUrl,
          apiKey: "embedded-local",
        }),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  React.useEffect(() => {
    const saved = localStorage.getItem("embedded-bootstrap-config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setForm((prev) => ({ ...prev, ...parsed }));
      } catch (_) {}
    }
  }, []);

  return (
    <Container size="2">
      <Heading align="center" as="h2" size="6" my="6">
        Start Embedded Agent
      </Heading>

      <Box mb="4">
        <Text size="2">
          Enter the runtime arguments required to launch the local LSP engine.
        </Text>
      </Box>

      <form onSubmit={onSubmit}>
        <Flex direction="column" gap="3">
          <Box>
            <Text as="label">Launcher URL</Text>
            <TextField.Root
              value={form.launcherUrl}
              onChange={onChange("launcherUrl")}
              placeholder="http://127.0.0.1:8009"
              required
            />
          </Box>

          <Box>
            <Text as="label">API server URL</Text>
            <TextField.Root
              value={form.addressUrl}
              onChange={onChange("addressUrl")}
              placeholder="http://127.0.0.1:8002"
              required
            />
          </Box>
          <Box>
            <Text as="label">API key (optional)</Text>
            <TextField.Root
              value={form.apiKey}
              onChange={onChange("apiKey")}
              placeholder="embedded-local"
              name="api-key"
              type="password"
            />
          </Box>

          <Box>
            <Text as="label">refact-lsp binary path</Text>
            <TextField.Root
              value={form.binaryPath}
              onChange={onChange("binaryPath")}
              placeholder="/path/to/refact-lsp"
              required
            />
          </Box>

          <Box>
            <Text as="label">Workspace path</Text>
            <TextField.Root
              value={form.workspacePath}
              onChange={onChange("workspacePath")}
              placeholder="/path/to/workspace"
              required
            />
          </Box>

          <Box>
            <Text as="label">VecDB path</Text>
            <TextField.Root
              value={form.vecDbPath}
              onChange={onChange("vecDbPath")}
              placeholder="/path/to/file.vecdb"
            />
          </Box>

          <Box>
            <Text as="label">ESP-IDF path</Text>
            <TextField.Root
              value={form.espIdfPath}
              onChange={onChange("espIdfPath")}
              placeholder="/path/to/esp-idf"
            />
          </Box>

          <Box>
            <Text as="label">Board definition</Text>
            <TextField.Root
              value={form.boardDefinition}
              onChange={onChange("boardDefinition")}
              placeholder="esp32-s3-devkitc-1-n32r8v"
              required
            />
          </Box>

          <Box>
            <Text as="label">Platform</Text>
            <TextField.Root
              value={form.platform}
              onChange={onChange("platform")}
              placeholder="esp32"
              required
            />
          </Box>

          <Box>
            <Text as="label">LSP HTTP port</Text>
            <TextField.Root
              value={form.httpPort}
              onChange={onChange("httpPort")}
              placeholder="8486"
              required
            />
          </Box>

          {error && (
            <Box>
              <Text color="red" size="2">
                {error}
              </Text>
            </Box>
          )}

          <Flex justify="end">
            <Button type="submit" loading={loading} disabled={loading}>
              Launch Agent
            </Button>
          </Flex>
        </Flex>
      </form>
    </Container>
  );
};