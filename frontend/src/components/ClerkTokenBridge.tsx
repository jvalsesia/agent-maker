import { useEffect } from "react";
import { useAuth } from "@clerk/clerk-react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { setAuthTokenGetter, setUnauthorizedHandler } from "@/lib/api";

/**
 * Bridges Clerk's React session into the framework-free `api.ts` fetch wrapper:
 * registers a bearer-token getter and a 401 handler at mount, and unregisters on
 * unmount. Renders nothing. Must live inside both `<ClerkProvider>` and the router.
 */
export function ClerkTokenBridge() {
  const { getToken } = useAuth();
  const navigate = useNavigate();
  const { t } = useTranslation();

  useEffect(() => {
    setAuthTokenGetter(() => getToken());
    setUnauthorizedHandler(() => {
      toast.error(t("auth.sessionExpired"));
      navigate("/sign-in", { replace: true });
    });
    return () => {
      setAuthTokenGetter(null);
      setUnauthorizedHandler(null);
    };
  }, [getToken, navigate, t]);

  return null;
}
