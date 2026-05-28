import { FormEvent, useRef, useState } from "react";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { AuthError, useLogin, useMe } from "@/hooks/useAuth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface LocationState {
  from?: string;
}

export function LoginPage() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();
  const { data: me } = useMe();
  const login = useLogin();

  const passwordRef = useRef<HTMLInputElement | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [errorCode, setErrorCode] = useState<string | null>(null);

  // Already authenticated: bounce away from /login.
  if (me) {
    const from = (location.state as LocationState | null)?.from;
    return <Navigate to={from && from !== "/login" ? from : "/"} replace />;
  }

  const expired = new URLSearchParams(location.search).get("expired") === "1";

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setErrorCode(null);
    try {
      await login.mutateAsync({ email, password });
      const from = (location.state as LocationState | null)?.from;
      navigate(from && from !== "/login" ? from : "/", { replace: true });
    } catch (err) {
      const code = err instanceof AuthError ? err.code : "invalid_credentials";
      setErrorCode(code);
      setPassword("");
      passwordRef.current?.focus();
    }
  };

  const error = errorCode
    ? errorCode === "auth_unavailable"
      ? t("auth.error.unavailable")
      : t("auth.error.invalidCredentials")
    : null;

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 p-6">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-sm space-y-5 rounded-lg border border-border bg-card p-6 shadow-sm"
      >
        <div>
          <h1 className="text-lg font-semibold">{t("auth.title")}</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {t("auth.subtitle")}
          </p>
        </div>

        {expired && !error && (
          <div className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-900 dark:text-amber-200">
            {t("auth.sessionExpired")}
          </div>
        )}

        {error && (
          <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </div>
        )}

        <div className="space-y-2">
          <Label htmlFor="auth-email">{t("auth.email")}</Label>
          <Input
            id="auth-email"
            type="email"
            autoComplete="email"
            autoFocus
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="auth-password">{t("auth.password")}</Label>
          <Input
            id="auth-password"
            ref={passwordRef}
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </div>

        <Button type="submit" className="w-full" disabled={login.isPending}>
          {login.isPending ? t("auth.signingIn") : t("auth.signIn")}
        </Button>
      </form>
    </div>
  );
}
