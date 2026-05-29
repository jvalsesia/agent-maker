import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Bot, Sparkles, BookOpen, MessageSquare, Settings } from "lucide-react";
import { SignedIn, UserButton, useUser } from "@clerk/clerk-react";
import { cn } from "@/lib/cn";
import { clerkEnabled } from "@/lib/clerk";

const items = [
  { to: "/agents", labelKey: "nav.agents", icon: Bot, disabled: false },
  { to: "/skills", labelKey: "nav.skills", icon: Sparkles, disabled: false },
  { to: "/templates", labelKey: "nav.templates", icon: BookOpen, disabled: false },
  { to: "/conversations", labelKey: "nav.conversations", icon: MessageSquare, disabled: true },
  { to: "/settings", labelKey: "nav.settings", icon: Settings, disabled: false },
] as const;

function AccountFooter() {
  const { t } = useTranslation();
  const { user } = useUser();
  const label =
    user?.primaryEmailAddress?.emailAddress ?? user?.fullName ?? t("auth.account");
  return (
    <div className="mt-auto flex items-center gap-2 border-t border-border px-2 pt-3">
      <UserButton />
      <span className="truncate text-xs text-muted-foreground" title={label}>
        {label}
      </span>
    </div>
  );
}

export function NavSidebar() {
  const { t } = useTranslation();
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-card/40 p-3">
      <div className="px-2 py-3 text-sm font-semibold tracking-tight">{t("common.appName")}</div>
      <nav className="mt-2 flex flex-col gap-1">
        {items.map(({ to, labelKey, icon: Icon, disabled }) =>
          disabled ? (
            <span
              key={to}
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground/60 cursor-not-allowed"
              title={t("common.comingSoon")}
            >
              <Icon className="h-4 w-4" />
              {t(labelKey)}
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
              {t(labelKey)}
            </NavLink>
          ),
        )}
      </nav>
      {clerkEnabled && (
        <SignedIn>
          <AccountFooter />
        </SignedIn>
      )}
    </aside>
  );
}
