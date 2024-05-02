export const VStack = ({
  gap = "0.5rem",
  children,
}: {
  gap?: string;
  children: React.ReactNode;
}) => (
  <div
    style={{
      display: "flex",
      flexDirection: "column",
      gap,
    }}
  >
    {children}
  </div>
);
