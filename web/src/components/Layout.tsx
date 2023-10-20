import {
  AppShell,
  Burger,
  Flex,
  Header,
  MediaQuery,
  Title,
  useMantineTheme,
} from "@mantine/core";
import { useState } from "react";
import { Outlet } from "react-router-dom";
import { NavigationBar } from "./NavigationBar";
import { SpotifyWidget } from "./SpotifyWidget";

export const Layout = () => {
  const theme = useMantineTheme();
  const [opened, setOpened] = useState(false);
  return (
    <AppShell
      navbarOffsetBreakpoint="sm"
      asideOffsetBreakpoint="sm"
      header={
        <Header
          height={50}
          p="md"
          style={{
            background:
              "linear-gradient(to bottom, rgb(25, 110, 150), rgb(6, 80, 120))",
            borderBottom: "none",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", height: "100%" }}
          >
            <MediaQuery largerThan="sm" styles={{ display: "none" }}>
              <Burger
                opened={opened}
                onClick={() => setOpened((o) => !o)}
                size="sm"
                color={theme.colors.gray[6]}
                mr="xl"
              />
            </MediaQuery>

            <Flex align="center" justify="space-between" style={{ flex: 1 }}>
              <Title
                order={1}
                weight="normal"
                sx={{
                  fontFamily: "YoungSerif",
                  letterSpacing: "-1.5px",
                  fontSize: "2rem",
                }}
                color="white"
              >
                `lute
              </Title>
              <SpotifyWidget />
            </Flex>
          </div>
        </Header>
      }
      navbar={<NavigationBar isOpen={opened} />}
      padding="xs"
    >
      <Outlet />
    </AppShell>
  );
};
