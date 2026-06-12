import React from "react";
import { Box, Button, Flex, Text } from "@radix-ui/themes";

import type { PirAnalyzeResult, PirStatus } from "../pirTypes";
import type { ValidationReport } from "../types";
import { PirAnalyzedFilesList } from "./PirAnalyzedFilesList";

export type TopologyApprovalCardProps = {
  pirStatus: PirStatus | null;
  result: PirAnalyzeResult | null;
  validation: ValidationReport | null;
  loading?: boolean;
  expanded?: boolean;
  onToggleExpand?: () => void;
  onOpenDiagram?: () => void;
  onRefresh?: () => void;
};

export const TopologyApprovalCard: React.FC<TopologyApprovalCardProps> = ({
  pirStatus,
  result,
  validation,
  loading,
  expanded = false,
  onToggleExpand,
  onOpenDiagram,
  onRefresh,
}) => {
  const approval =
    result?.pir.approval.status ?? pirStatus?.approval_status ?? "pending";
  const headline =
    result?.pir.summary?.headline ??
    pirStatus?.summary_headline ??
    "Firmware topology";
  const nodeCount = result?.graph.nodes.length ?? 0;
  const isAnalyzing = (loading ?? false) || pirStatus?.status === "analyzing";
  const analyzedFiles = result?.pir.provenance.analyzed_files ?? [];

  return (
    <Box
      p="2"
      style={{
        border: "1px solid var(--gray-6)",
        borderRadius: 8,
        background: "var(--gray-2)",
      }}
    >
      <Flex direction="column" gap="2">
        <Flex justify="between" align="start" gap="2" wrap="wrap">
          <Flex direction="column" gap="1" style={{ minWidth: 0, flex: 1 }}>
            <Text size="2" weight="bold">
              Firmware topology
            </Text>
            <Text size="1" color="gray" style={{ lineHeight: 1.4 }}>
              {isAnalyzing
                ? "Updating diagram from project sources…"
                : headline}
            </Text>
          </Flex>
          <Flex gap="1" wrap="wrap" align="center" style={{ flexShrink: 0 }}>
            {isAnalyzing ? (
              <Text size="1" color="gray">
                Analyzing…
              </Text>
            ) : nodeCount > 0 ? (
              <Text size="1" color="gray">
                {nodeCount} node{nodeCount === 1 ? "" : "s"}
              </Text>
            ) : null}
            <Text size="1">
              <strong>{approval}</strong>
              {result?.pir.revision
                ? ` · ${result.pir.revision.slice(0, 8)}`
                : null}
            </Text>
          </Flex>
        </Flex>


        {result?.diff &&
        (result.diff.nodes_added.length > 0 ||
          result.diff.nodes_removed.length > 0) ? (
          <Text size="1" color="gray">
            Diff: +{result.diff.nodes_added.length} / -{result.diff.nodes_removed.length}{" "}
            nodes
          </Text>
        ) : null}

        {analyzedFiles.length > 0 ? (
          <PirAnalyzedFilesList files={analyzedFiles} compact defaultOpen={expanded} />
        ) : null}

        <Flex gap="2" wrap="wrap" align="center">
          {onToggleExpand ? (
            <Button size="1" variant="soft" onClick={onToggleExpand}>
              {expanded ? "Collapse diagram" : "Expand diagram"}
            </Button>
          ) : null}
          {onRefresh ? (
            <Button size="1" variant="ghost" onClick={onRefresh} disabled={isAnalyzing}>
              Refresh
            </Button>
          ) : null}
          {expanded && onOpenDiagram ? (
            <Button size="1" variant="ghost" onClick={onOpenDiagram}>
              Open full page
            </Button>
          ) : null}
        </Flex>
      </Flex>
    </Box>
  );
};
