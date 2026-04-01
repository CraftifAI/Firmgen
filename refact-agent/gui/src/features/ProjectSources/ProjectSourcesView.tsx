import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  Box,
  Button,
  Flex,
  Text,
  Spinner,
} from "@radix-ui/themes";
import { UploadIcon } from "@radix-ui/react-icons";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { pop } from "../Pages/pagesSlice";
import {
  formatBytes,
  listProjectSources,
  uploadProjectSources,
  type ProjectSourceFile,
} from "../../services/refact/projectSources";

type Props = {
  projectId: string;
};

export const ProjectSourcesView: React.FC<Props> = ({ projectId }) => {
  const dispatch = useAppDispatch();
  const project = useAppSelector((s) =>
    s.workspaceProjects.projects.find((p) => p.id === projectId),
  );
  const lspPort = useAppSelector((s) => s.config.lspPort);

  const inputRef = useRef<HTMLInputElement>(null);
  const [files, setFiles] = useState<ProjectSourceFile[]>([]);
  const [directory, setDirectory] = useState<string>("");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [uploadBusy, setUploadBusy] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [uploadOk, setUploadOk] = useState<string | null>(null);

  const projectRoot = project?.esp32_projects_path?.trim() ?? "";

  const refresh = useCallback(async () => {
    if (!projectRoot) return;
    setLoading(true);
    setLoadError(null);
    try {
      const res = await listProjectSources(projectRoot, lspPort);
      setFiles(res.files);
      setDirectory(res.directory);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : String(e));
      setFiles([]);
      setDirectory("");
    } finally {
      setLoading(false);
    }
  }, [projectRoot, lspPort]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const onPickFiles = () => inputRef.current?.click();

  const onFilesSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const list = e.target.files;
    if (!list?.length || !projectRoot) return;
    setUploadError(null);
    setUploadOk(null);
    setUploadBusy(true);
    try {
      const arr = Array.from(list);
      const res = await uploadProjectSources(projectRoot, arr, lspPort);
      setUploadOk(`Saved: ${res.saved.join(", ")}`);
      await refresh();
    } catch (err) {
      setUploadError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploadBusy(false);
      e.target.value = "";
    }
  };

  if (!project) {
    return (
      <Flex direction="column" align="center" justify="center" p="6" gap="3">
        <Text color="gray">Unknown project.</Text>
        <Button variant="soft" onClick={() => dispatch(pop())}>
          Back
        </Button>
      </Flex>
    );
  }

  if (!projectRoot) {
    return (
      <Flex direction="column" align="center" justify="center" p="6" gap="3">
        <Text color="gray">This project has no workspace path.</Text>
        <Button variant="soft" onClick={() => dispatch(pop())}>
          Back
        </Button>
      </Flex>
    );
  }

  return (
    <Flex
      direction="column"
      gap="4"
      p="6"
      style={{ minHeight: "60vh", maxWidth: 640, margin: "0 auto", width: "100%" }}
    >
      <Flex justify="between" align="center" gap="3" wrap="wrap">
        <Box>
          <Text size="5" weight="bold">
            Sources
          </Text>
          <Text size="3" color="gray" as="div" mt="1">
            {project.name}
          </Text>
        </Box>
        <Button variant="soft" onClick={() => dispatch(pop())}>
          Back
        </Button>
      </Flex>

      <Text size="2" color="gray">
        Files are stored on disk under your project folder in a{" "}
        <Text weight="bold" size="2" as="span">
          sources
        </Text>{" "}
        subfolder (alongside ESP-IDF apps).
      </Text>

      <Box
        p="3"
        style={{
          borderRadius: 8,
          border: "1px dashed color-mix(in srgb, var(--gray-a8) 50%, transparent)",
          background: "var(--color-panel-translucent)",
        }}
      >
        <Text size="1" color="gray" style={{ fontFamily: "var(--font-mono)", wordBreak: "break-all" }}>
          {directory || projectRoot}
        </Text>
      </Box>

      <input
        ref={inputRef}
        type="file"
        multiple
        style={{ display: "none" }}
        onChange={onFilesSelected}
      />

      <Flex gap="2" align="center" wrap="wrap">
        <Button onClick={onPickFiles} disabled={uploadBusy}>
          {uploadBusy ? (
            <Flex align="center" gap="2">
              <Spinner />
              Uploading…
            </Flex>
          ) : (
            <Flex align="center" gap="2">
              <UploadIcon />
              Add files…
            </Flex>
          )}
        </Button>
        <Button variant="outline" onClick={() => void refresh()} disabled={loading || uploadBusy}>
          Refresh list
        </Button>
      </Flex>

      {uploadOk && (
        <Text size="2" style={{ color: "var(--green-11)" }}>
          {uploadOk}
        </Text>
      )}
      {uploadError && (
        <Text size="2" color="red">
          {uploadError}
        </Text>
      )}
      {loadError && (
        <Text size="2" color="red">
          {loadError}
        </Text>
      )}

      <Text size="2" weight="medium">
        Files in sources
      </Text>
      {loading ? (
        <Flex align="center" gap="2">
          <Spinner />
          <Text size="2" color="gray">
            Loading…
          </Text>
        </Flex>
      ) : files.length === 0 ? (
        <Text size="2" color="gray">
          No files yet. Use Add files to upload PDFs, notes, or code.
        </Text>
      ) : (
        <ul style={{ margin: 0, paddingLeft: "1.25rem" }}>
          {files.map((f) => (
            <li key={f.name}>
              <Text size="2">
                {f.name}{" "}
                <Text size="1" color="gray" as="span">
                  ({formatBytes(f.size_bytes)})
                </Text>
              </Text>
            </li>
          ))}
        </ul>
      )}
    </Flex>
  );
};
