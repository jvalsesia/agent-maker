import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { useSettings } from "@/hooks/useSettings";
import { useApplyTheme } from "@/hooks/useTheme";

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

function Gate({ children }: { children: React.ReactNode }) {
  const { data, isLoading, error } = useSettings();
  useApplyTheme(data?.appearance.theme);

  if (isLoading) {
    return <div className="flex h-screen items-center justify-center text-sm text-muted-foreground">Loading…</div>;
  }
  if (error) {
    return (
      <div className="mx-auto mt-24 max-w-xl text-center space-y-3">
        <h1 className="text-xl font-semibold">Database unavailable</h1>
        <p className="text-sm text-muted-foreground">
          The backend cannot reach Postgres. Start it with:
        </p>
        <pre className="rounded-md border border-border bg-muted p-3 text-left text-xs">docker compose up -d</pre>
        <button
          onClick={() => location.reload()}
          className="text-sm underline text-foreground"
        >
          Retry
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
        <Route path="/onboarding" element={<Gate><OnboardingPage /></Gate>} />
        <Route element={<Gate><Layout /></Gate>}>
          <Route path="/" element={<Navigate to="/agents" replace />} />
          <Route path="/agents" element={<AgentsList />} />
          <Route path="/agents/new" element={<AgentForm />} />
          <Route path="/agents/:id" element={<AgentForm />} />
          <Route path="/skills" element={<SkillsList />} />
          <Route path="/skills/new" element={<SkillForm />} />
          <Route path="/skills/:id" element={<SkillForm />} />
          <Route path="/templates" element={<TemplatesGallery />} />
          <Route path="/settings/*" element={<SettingsPage />} />
        </Route>
      </Routes>
    </Suspense>
  );
}
