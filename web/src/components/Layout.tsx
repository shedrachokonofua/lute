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
import { NavigationProgressBar } from "./NavigationProgressBar";
import { SpotifyWidget } from "./SpotifyWidget";

export const Layout = () => {
  const theme = useMantineTheme();
  const [opened, setOpened] = useState(false);
  return (
    <div>
      <NavigationProgressBar />
      <AppShell
        navbarOffsetBreakpoint="sm"
        asideOffsetBreakpoint="sm"
        header={
          <Header
            height={50}
            p="md"
            style={{
              background: "#1F2D5C",
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
        padding={0}
      >
        <Outlet />
      </AppShell>
    </div>
  );
};
