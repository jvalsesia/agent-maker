import { NavLink } from "react-router-dom";
import { Bot, Sparkles, BookOpen, MessageSquare, Settings } from "lucide-react";
import { cn } from "@/lib/cn";

const items = [
  { to: "/agents", label: "Agents", icon: Bot, disabled: false },
  { to: "/skills", label: "Skills", icon: Sparkles, disabled: false },
  { to: "/templates", label: "Templates", icon: BookOpen, disabled: true },
  { to: "/conversations", label: "Conversations", icon: MessageSquare, disabled: true },
  { to: "/settings", label: "Settings", icon: Settings, disabled: false },
];

export function NavSidebar() {
  return (
    <aside className="w-56 shrink-0 border-r border-border bg-card/40 p-3">
      <div className="px-2 py-3 text-sm font-semibold tracking-tight">agent-maker</div>
      <nav className="mt-2 flex flex-col gap-1">
        {items.map(({ to, label, icon: Icon, disabled }) =>
          disabled ? (
            <span
              key={to}
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground/60 cursor-not-allowed"
              title="Coming soon"
            >
              <Icon className="h-4 w-4" />
              {label}
            </span>
          ) : (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-accent text-accent-foreground"
                    : "text-foreground/80 hover:bg-accent hover:text-accent-foreground",
                )
              }
            >
              <Icon className="h-4 w-4" />
              {label}
            </NavLink>
          ),
        )}
      </nav>
    </aside>
  );
}
