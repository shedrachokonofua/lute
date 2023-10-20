import { Navbar, NavLink } from "@mantine/core";

const NavItem = ({
  label,
  href,
  active,
}: {
  label: string;
  href: string;
  active?: boolean;
}) => (
  <NavLink
    component="a"
    href={href}
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

const items = [
  { label: "Dashboard", href: "/" },
  { label: "Profiles", href: "/profiles" },
  { label: "Recommendations", href: "/recommendations" },
];

export const NavigationBar = ({ isOpen }: { isOpen: boolean }) => {
  const navItems = items.map((item) => ({
    ...item,
    active: window.location.pathname === item.href,
  }));
  return (
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
};
