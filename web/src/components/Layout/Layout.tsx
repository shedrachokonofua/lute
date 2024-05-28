import {
  AppShell,
  Burger,
  Flex,
  NavLink,
  Title,
  useMantineTheme,
} from "@mantine/core";
import { useState } from "react";
import { Link, Outlet, useMatch } from "react-router-dom";
import classes from "./Layout.module.css";
import { NavigationProgressBar } from "./NavigationProgressBar";
import { SpotifyAuthWidget } from "./SpotifyAuthWidget";

const NavItem = ({
  label,
  href,
  activePath,
}: {
  label: string;
  href: string;
  activePath?: string;
}) => {
  const active = Boolean(useMatch(activePath || href));
  return (
    <NavLink
      component={Link}
      to={href}
      label={label}
      className={classes.navItem}
      variant="light"
      active={active}
    />
  );
};

const navItems = [
  { label: "Dashboard", href: "/" },
  { label: "Profiles", href: "/profiles", activePath: "/profiles/*" },
  { label: "Recommendations", href: "/recommendations" },
  { label: "Find Similar Albums", href: "/similar-albums" },
];

export const HEADER_HEIGHT = 50;

export const Layout = () => {
  const theme = useMantineTheme();
  const [opened, setOpened] = useState(false);
  return (
    <div>
      <NavigationProgressBar />
      <AppShell
        header={{
          height: HEADER_HEIGHT,
        }}
        navbar={{
          width: 200,
          breakpoint: "sm",
          collapsed: { mobile: !opened },
        }}
        padding={0}
      >
        <AppShell.Header
          p="md"
          style={{
            background: "#1F2D5C",
            borderBottom: "none",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", height: "100%" }}
          >
            <Burger
              opened={opened}
              onClick={() => setOpened((o) => !o)}
              size="sm"
              color={theme.colors.gray[6]}
              mr="xl"
              hiddenFrom="sm"
            />

            <Flex align="center" justify="space-between" style={{ flex: 1 }}>
              <Title
                order={1}
                fw="normal"
                style={{
                  fontFamily: "YoungSerif",
                  letterSpacing: "-1.5px",
                  fontSize: "2rem",
                }}
                c="white"
              >
                `lute
              </Title>
              <SpotifyAuthWidget />
            </Flex>
          </div>
        </AppShell.Header>
        <AppShell.Navbar>
          {navItems.map((item) => (
            <NavItem key={item.href} {...item} />
          ))}
        </AppShell.Navbar>
        <AppShell.Main>
          <Outlet />
        </AppShell.Main>
      </AppShell>
    </div>
  );
};
