import React, { useCallback } from "react";

import { Flex, Text, Link } from "@radix-ui/themes";

import { useConfig, useAppSelector, useEventsBusForIDE } from "../../hooks";

// import { currentTipOfTheDay } from "../../features/TipOfTheDay";

const TipOfTheDay: React.FC = () => {
  // const tip = useAppSelector(currentTipOfTheDay);
  return (
    <Text>
      {/* 💡 <b>Tip of the day</b>: {tip} */}
    </Text>
  );
};

export const PlaceHolderText: React.FC = () => {
  const config = useConfig();
  const hasVecDB = config.features?.vecdb ?? false;
  const hasAst = config.features?.ast ?? false;
  const { openSettings } = useEventsBusForIDE();

  const handleOpenSettings = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      openSettings();
    },
    [openSettings],
  );

  const welcomeHeading = (
    <Flex direction="column" align="center" gap="2" style={{ width: "100%" }}>
      <h2
        style={{
          fontSize: "2rem",
          fontWeight: 300,
          color: "var(--gray-12)",
          textAlign: "center",
          margin: 0,
          letterSpacing: "-0.01em",
        }}
      >
        Ask FirmGen
      </h2>
      <Text size="2" style={{ color: "var(--gray-9)" }}>
        Your AI for Firmware Generation
      </Text>
    </Flex>
  );

  if (config.host === "web") {
    return (
      <Flex direction="column" gap="4">
        {welcomeHeading}
        <TipOfTheDay />
      </Flex>
    );
  }

  if (!hasVecDB && !hasAst) {
    return (
      <Flex direction="column" gap="4">
        {welcomeHeading}
        <Text>
          💡 You can turn on VecDB and AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasVecDB) {
    return (
      <Flex direction="column" gap="4">
        {welcomeHeading}
        <Text>
          💡 You can turn on VecDB in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasAst) {
    return (
      <Flex direction="column" gap="4">
        {welcomeHeading}
        <Text>
          💡 You can turn on AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  }

  return (
    <Flex direction="column" gap="4">
      {welcomeHeading}
      {/* <TipOfTheDay /> */}
    </Flex>
  );
};
