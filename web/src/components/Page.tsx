import { HEADER_HEIGHT } from "./Layout";

export const Page = ({
  children,
  style,
}: {
  children: React.ReactNode;
  style?: React.CSSProperties;
}) => (
  <div
    style={{
      height: `calc(100vh - ${HEADER_HEIGHT}px)`,
      overflowY: "auto",
      display: "flex",
      flexDirection: "column",
      ...style,
    }}
  >
    {children}
  </div>
);
