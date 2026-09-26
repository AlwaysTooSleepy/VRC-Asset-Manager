export function Pill({ children, variant = "site" }: { children: React.ReactNode; variant?: "site" | "category" }) {
  return <span className={`pill ${variant === "category" ? "pill-category" : ""}`}>{children}</span>;
}
