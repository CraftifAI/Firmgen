import React, { useState, useCallback } from "react";
import { Card, Flex, Heading, Text, TextField, Button } from "@radix-ui/themes";

type EmbeddedBootstrapPageProps = {
  showHeading?: boolean;
  onLaunched?: (payload: { workspacePath: string }) => void;
};

export const EmbeddedBootstrapPage: React.FC<EmbeddedBootstrapPageProps> = ({
  showHeading = true,
  onLaunched,
}) => {
  const [workspacePath, setWorkspacePath] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleConfirm = useCallback(() => {
    const trimmed = workspacePath.trim();
    if (!trimmed) {
      setError("Please enter a folder path.");
      return;
    }
    setError(null);
    onLaunched?.({ workspacePath: trimmed });
  }, [workspacePath, onLaunched]);

  return (
    <Card>
      <Flex direction="column" gap="2">
        {showHeading && (
          <Heading as="h3" size="3">
            Select local workspace folder
          </Heading>
        )}
        <Text size="2" color="gray">
          Enter the local folder path you want the agent to use as your
          workspace. This will be sent to the embedded agent as a project root.
        </Text>
        <TextField.Root
          placeholder="/path/to/your/project"
          value={workspacePath}
          onChange={(event) => setWorkspacePath(event.target.value)}
        />
        {error && (
          <Text size="1" color="red">
            {error}
          </Text>
        )}
        <Flex justify="end">
          <Button onClick={handleConfirm}>Use this folder</Button>
        </Flex>
      </Flex>
    </Card>
  );
}

