import React, { useEffect, useState, useCallback } from "react";
import {
  Box,
  Text,
  Flex,
  Card,
  Progress,
  Badge,
} from "@radix-ui/themes";
import { useAppSelector, useAppDispatch } from "../../../hooks";
import { useConfig } from "../../../hooks";
import { setVecDbStatus, selectVecDbStatus } from "../../../features/Knowledge/knowledgeSlice";
import { isVecDbStatus } from "../../../services/refact/types";
import styles from "./VecDbStatusPanel.module.css";

export const VecDbStatusPanel: React.FC = () => {
  const vecDbStatus = useAppSelector(selectVecDbStatus);
  const config = useConfig();
  const dispatch = useAppDispatch();
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [isLoading, setIsLoading] = useState(false);

  // Fetch and update VecDB status
  const fetchVecDbStatus = useCallback(async () => {
    try {
      setIsLoading(true);
      const port = config.lspPort;
      const response = await fetch(`http://127.0.0.1:${port}/v1/vdb-status`);
      if (response.ok) {
        const data = await response.json();
        // Check if VecDB is turned off
        if (data.detail === "turned_off") {
          // VecDB is not enabled, don't update status
          setIsLoading(false);
          return;
        }
        // Validate and update Redux state
        if (isVecDbStatus(data)) {
          dispatch(setVecDbStatus(data));
          setLastUpdate(new Date());
        }
      }
    } catch (error) {
      console.error("Failed to fetch VecDB status:", error);
    } finally {
      setIsLoading(false);
    }
  }, [config.lspPort, dispatch]);

  // Initial fetch and polling
  useEffect(() => {
    // Fetch immediately
    fetchVecDbStatus();

    // Then poll every 2 seconds
    const interval = setInterval(() => {
      fetchVecDbStatus();
    }, 2000);

    return () => clearInterval(interval);
  }, [fetchVecDbStatus]);

  const getStateColor = (state: string) => {
    switch (state) {
      case "done":
        return "green";
      case "parsing":
        return "blue";
      case "starting":
        return "yellow";
      case "cooldown":
        return "orange";
      default:
        return "gray";
    }
  };

  const getProgress = () => {
    if (!vecDbStatus || vecDbStatus.files_total === 0) return 0;
    const processed = vecDbStatus.files_total - vecDbStatus.files_unprocessed;
    return (processed / vecDbStatus.files_total) * 100;
  };

  if (!vecDbStatus) {
    return (
      <Card className={styles.container}>
        <Flex direction="column" height="100%">
          <Text size="2" weight="bold" mb="2">
            VecDB Status
          </Text>
          <Text size="2" color="gray" style={{ fontStyle: "italic" }}>
            VecDB status not available. Enable VecDB in settings.
          </Text>
        </Flex>
      </Card>
    );
  }

  const progress = getProgress();
  const hasErrors = Object.keys(vecDbStatus.vecdb_errors || {}).length > 0;

  return (
    <Card className={styles.container}>
      <Flex direction="column" height="100%" gap="2">
        <Flex align="center" justify="between" pb="2" style={{ borderBottom: "1px solid var(--gray-6)" }}>
          <Text size="2" weight="bold">
            VecDB Status
          </Text>
          <Badge color={getStateColor(vecDbStatus.state)} variant="soft">
            {vecDbStatus.state.toUpperCase()}
          </Badge>
        </Flex>

        <Box>
          <Flex justify="between" align="center" mb="1">
            <Text size="2" weight="medium">
              Indexing Progress
            </Text>
            <Text size="1" color="gray">
              {vecDbStatus.files_total > 0
                ? `${vecDbStatus.files_total - vecDbStatus.files_unprocessed} / ${vecDbStatus.files_total} files`
                : "N/A"}
            </Text>
          </Flex>
          {vecDbStatus.files_total > 0 && (
            <Progress value={progress} className={styles.progress} />
          )}
        </Box>

        <Box className={styles.statsGrid}>
          <Box>
            <Text size="1" color="gray">
              Files Unprocessed
            </Text>
            <Text size="3" weight="bold">
              {vecDbStatus.files_unprocessed}
            </Text>
          </Box>
          <Box>
            <Text size="1" color="gray">
              Vectors Created
            </Text>
            <Text size="3" weight="bold">
              {vecDbStatus.vectors_made_since_start.toLocaleString()}
            </Text>
          </Box>
          <Box>
            <Text size="1" color="gray">
              Requests Made
            </Text>
            <Text size="3" weight="bold">
              {vecDbStatus.requests_made_since_start}
            </Text>
          </Box>
          <Box>
            <Text size="1" color="gray">
              DB Size
            </Text>
            <Text size="3" weight="bold">
              {vecDbStatus.db_size.toLocaleString()}
            </Text>
          </Box>
        </Box>

        {vecDbStatus.queue_additions && (
          <Box className={styles.infoBox}>
            <Text size="1" color="blue">
              ⚠️ Queue additions enabled
            </Text>
          </Box>
        )}

        {vecDbStatus.vecdb_max_files_hit && (
          <Box className={styles.warningBox}>
            <Text size="1" color="orange">
              ⚠️ Max files limit reached
            </Text>
          </Box>
        )}

        {hasErrors && (
          <Box className={styles.errorBox}>
            <Text size="1" weight="bold" color="red" mb="1">
              Errors:
            </Text>
            <Box className={styles.errorsList}>
              {Object.entries(vecDbStatus.vecdb_errors).map(([file, count]) => (
                <Text key={file} size="1" color="red">
                  {file}: {count} error{count !== 1 ? "s" : ""}
                </Text>
              ))}
            </Box>
          </Box>
        )}

        <Text size="1" color="gray" style={{ fontStyle: "italic", marginTop: "auto" }}>
          Last updated: {lastUpdate.toLocaleTimeString()}
        </Text>
      </Flex>
    </Card>
  );
};

