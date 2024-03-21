import { Navbar, NavLink } from "@mantine/core";
import { Link, useMatch } from "react-router-dom";

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
};

const navItems = [
  { label: "Dashboard", href: "/" },
  { label: "Profiles", href: "/profiles", activePath: "/profiles/*" },
  { label: "Recommendations", href: "/recommendations" },
  { label: "Find Similar Albums", href: "/similar-albums" },
];

export const NavigationBar = ({ isOpen }: { isOpen: boolean }) => (
  <Navbar
    p="md"
    hiddenBreakpoint="sm"
    hidden={!isOpen}
    width={{ base: 200 }}
    style={{
      background: "rgb(25, 25, 25)",
      color: "rgb(200, 200, 200)",
      border: "none",
      padding: "0.5rem",
    }}
  >
    {navItems.map((item) => (
      <NavItem key={item.href} {...item} />
    ))}
  </Navbar>
);
