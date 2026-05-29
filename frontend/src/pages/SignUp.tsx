import { SignUp } from "@clerk/clerk-react";
import { useTranslation } from "react-i18next";

export function SignUpPage() {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-6 bg-background p-6">
      <div className="text-center space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t("common.appName")}</h1>
        <p className="text-sm text-muted-foreground">{t("auth.signUpSubtitle")}</p>
      </div>
      <SignUp routing="path" path="/sign-up" signInUrl="/sign-in" />
    </div>
  );
}
