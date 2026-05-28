import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Layout } from "@/components/Layout";
import { RequireAuth } from "@/components/auth/RequireAuth";
import { useSettings } from "@/hooks/useSettings";
import { useApplyTheme } from "@/hooks/useTheme";
import { applyLocale } from "@/hooks/useLocale";

const OnboardingPage = lazy(() =>
  import("@/pages/Onboarding").then((m) => ({ default: m.OnboardingPage })),
);
const SettingsPage = lazy(() =>
  import("@/pages/Settings").then((m) => ({ default: m.SettingsPage })),
);
const AgentsList = lazy(() =>
  import("@/pages/Agents").then((m) => ({ default: m.AgentsList })),
);
const AgentForm = lazy(() =>
  import("@/pages/Agents").then((m) => ({ default: m.AgentForm })),
);
const SkillsList = lazy(() =>
  import("@/pages/Skills").then((m) => ({ default: m.SkillsList })),
);
const SkillForm = lazy(() =>
  import("@/pages/Skills").then((m) => ({ default: m.SkillForm })),
);
const TemplatesGallery = lazy(() =>
  import("@/pages/Templates").then((m) => ({ default: m.TemplatesGallery })),
);
const ChatPage = lazy(() =>
  import("@/pages/Chat").then((m) => ({ default: m.ChatPage })),
);
const LoginPage = lazy(() =>
  import("@/pages/Login").then((m) => ({ default: m.LoginPage })),
);

function Gate({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation();
  const { data, isLoading, error } = useSettings();
  useApplyTheme(data?.appearance.theme);
  if (data?.appearance.locale) applyLocale(data.appearance.locale);

  if (isLoading) {
    return <div className="flex h-screen items-center justify-center text-sm text-muted-foreground">{t("common.loading")}</div>;
  }
  if (error) {
    return (
      <div className="mx-auto mt-24 max-w-xl text-center space-y-3">
        <h1 className="text-xl font-semibold">{t("gate.dbUnavailableTitle")}</h1>
        <p className="text-sm text-muted-foreground">
          {t("gate.dbUnavailableBody")}
        </p>
        <pre className="rounded-md border border-border bg-muted p-3 text-left text-xs">docker compose up -d</pre>
        <button
          onClick={() => location.reload()}
          className="text-sm underline text-foreground"
        >
          {t("common.retry")}
        </button>
      </div>
    );
  }
  const anyConfigured = data?.providers.some((p) => p.key_configured) ?? false;
  if (!anyConfigured && location.pathname !== "/onboarding") {
    return <Navigate to="/onboarding" replace />;
  }
  return <>{children}</>;
}

export function AppRoutes() {
  return (
    <Suspense fallback={<div className="p-6 text-sm text-muted-foreground">Loading…</div>}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/onboarding" element={<RequireAuth><Gate><OnboardingPage /></Gate></RequireAuth>} />
        <Route element={<RequireAuth><Gate><Layout /></Gate></RequireAuth>}>
          <Route path="/" element={<Navigate to="/agents" replace />} />
          <Route path="/agents" element={<AgentsList />} />
          <Route path="/agents/new" element={<AgentForm />} />
          <Route path="/agents/:id" element={<AgentForm />} />
          <Route path="/skills" element={<SkillsList />} />
          <Route path="/skills/new" element={<SkillForm />} />
          <Route path="/skills/:id" element={<SkillForm />} />
          <Route path="/templates" element={<TemplatesGallery />} />
          <Route path="/agents/:id/chat" element={<ChatPage />} />
          <Route path="/settings/*" element={<SettingsPage />} />
        </Route>
      </Routes>
    </Suspense>
  );
}
