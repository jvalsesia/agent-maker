import { NavLink, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Bot, Sparkles, BookOpen, MessageSquare, Settings, LogOut } from "lucide-react";
import { cn } from "@/lib/cn";
import { useLogout, useMe } from "@/hooks/useAuth";

const items = [
  { to: "/agents", labelKey: "nav.agents", icon: Bot, disabled: false },
  { to: "/skills", labelKey: "nav.skills", icon: Sparkles, disabled: false },
  { to: "/templates", labelKey: "nav.templates", icon: BookOpen, disabled: false },
  { to: "/conversations", labelKey: "nav.conversations", icon: MessageSquare, disabled: true },
  { to: "/settings", labelKey: "nav.settings", icon: Settings, disabled: false },
] as const;

export function NavSidebar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: me } = useMe();
  const logout = useLogout();

  const onSignOut = async () => {
    await logout.mutateAsync();
    navigate("/login", { replace: true });
  };

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

      {me && (
        <div className="mt-auto border-t border-border pt-3">
          <div className="px-2 pb-1 text-xs text-muted-foreground">
            {t("auth.signedInAs")}
          </div>
          <div
            className="px-2 pb-2 text-sm font-medium truncate"
            title={me.email}
          >
            {me.email}
          </div>
          <button
            type="button"
            onClick={onSignOut}
            disabled={logout.isPending}
            className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-foreground/80 transition-colors hover:bg-accent hover:text-accent-foreground disabled:opacity-50"
          >
            <LogOut className="h-4 w-4" />
            {logout.isPending ? t("auth.signingOut") : t("auth.signOut")}
          </button>
        </div>
      )}
    </aside>
  );
}
