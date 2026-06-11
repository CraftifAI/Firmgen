import React, { useMemo, useState } from "react";
import { Box, Flex, Text } from "@radix-ui/themes";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

import styles from "./PirAnalyzedFilesList.module.css";

export type PirAnalyzedFilesListProps = {
  files: string[];
  /** Smaller typography for inline chat block. */
  compact?: boolean;
  defaultOpen?: boolean;
};

function sortAnalyzedFiles(files: string[]): string[] {
  const priority = (f: string): number => {
    const n = f.replace(/\\/g, "/").toLowerCase();
    if (n.endsWith("app_config.h")) return 0;
    if (n.includes("/main/")) return 1;
    if (n.includes("sdkconfig")) return 2;
    if (n.includes("cmake")) return 3;
    return 4;
  };
  return [...files].sort((a, b) => {
    const pa = priority(a);
    const pb = priority(b);
    if (pa !== pb) return pa - pb;
    return a.localeCompare(b);
  });
}

export const PirAnalyzedFilesList: React.FC<PirAnalyzedFilesListProps> = ({
  files,
  compact = false,
  defaultOpen = false,
}) => {
  const [open, setOpen] = useState(defaultOpen);
  const sorted = useMemo(() => sortAnalyzedFiles(files), [files]);

  if (sorted.length === 0) return null;

  return (
    <Collapsible.Root open={open} onOpenChange={setOpen} className={styles.root}>
      <Collapsible.Trigger asChild>
        <button type="button" className={styles.trigger} aria-expanded={open}>
          <Text size={compact ? "1" : "2"} weight="medium">
            Source files read ({sorted.length})
          </Text>
          <ChevronDownIcon
            className={classNames(styles.chevron, { [styles.chevronOpen]: open })}
          />
        </button>
      </Collapsible.Trigger>
      <Collapsible.Content className={styles.content}>
        <Text size="1" color="gray" mb="2" as="p">
          PIR reads only the allowlisted ESP-IDF topology files below (not the whole tree).
          Inspector edits write to <code>main/app_config.h</code> when present.
          Sdkconfig files are included only when needed and loaded in filtered form.
        </Text>
        <Box className={styles.fileList} asChild>
          <ul>
            {sorted.map((file) => {
              const isPrimary = file.replace(/\\/g, "/").toLowerCase().endsWith("app_config.h");
              return (
                <li key={file} className={isPrimary ? styles.filePrimary : undefined}>
                  <Flex gap="2" align="center" wrap="wrap">
                    <Text size="1" className={styles.filePath}>
                      {file}
                    </Text>
                    {isPrimary ? (
                      <Text size="1" color="green" className={styles.badge}>
                        primary
                      </Text>
                    ) : null}
                  </Flex>
                </li>
              );
            })}
          </ul>
        </Box>
      </Collapsible.Content>
    </Collapsible.Root>
  );
};
