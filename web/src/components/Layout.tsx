import {
  AppShell,
  Burger,
  Flex,
  Header,
  MediaQuery,
  NavLink,
  Navbar,
  Title,
  useMantineTheme,
} from "@mantine/core";
import React, { useState } from "react";
import { Outlet } from "react-router-dom";
import { SpotifyWidget } from "./SpotifyWidget";

interface NavItemProps {
  label: string;
  href: string;
  active?: boolean;
}

const NavItem = ({ label, href, active }: NavItemProps) => (
  <NavLink
    component="a"
    href="/"
    label="Recommendations"
    sx={{
      color: "rgb(200, 200, 200)",
      "&:hover": {
        background: "rgb(35, 35, 35)",
      },
    }}
    variant="filled"
    color="dark"
    active={active}
  />
);

export const Layout = ({ children }: React.FC) => {
  const theme = useMantineTheme();
  const [opened, setOpened] = useState(false);
  return (
    <AppShell
      navbarOffsetBreakpoint="sm"
      asideOffsetBreakpoint="sm"
      navbar={
        <Navbar
          p="md"
          hiddenBreakpoint="sm"
          hidden={!opened}
          width={{ base: 200 }}
          style={{
            background: "rgb(25, 25, 25)",
            color: "rgb(200, 200, 200)",
            border: "none",
            padding: "0.5rem",
          }}
        >
          <NavItem label="Recommendations" href="/" active />
        </Navbar>
      }
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
      padding="xs"
    >
      <Outlet />
    </AppShell>
  );
};
