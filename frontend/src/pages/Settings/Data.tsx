import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation, Trans } from "react-i18next";
import { toast } from "sonner";
import { ApiError, api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogClose,
} from "@/components/ui/dialog";
import { settingsKey } from "@/hooks/useSettings";

export function DataSection() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [confirm, setConfirm] = useState("");
  const [open, setOpen] = useState(false);

  const wipe = async () => {
    try {
      await api.wipe();
      toast.success(t("settings.data.wiped"));
      setConfirm("");
      setOpen(false);
      await qc.invalidateQueries({ queryKey: settingsKey });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("settings.data.title")}</CardTitle>
        <CardDescription>{t("settings.data.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger asChild>
            <Button variant="destructive">{t("settings.data.wipeButton")}</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t("settings.data.dialogTitle")}</DialogTitle>
              <DialogDescription>
                <Trans
                  i18nKey="settings.data.dialogBody"
                  components={{ code: <code className="font-mono" /> }}
                />
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-1">
              <Label htmlFor="confirm">{t("settings.data.confirmLabel")}</Label>
              <Input id="confirm" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button variant="outline">{t("common.cancel")}</Button>
              </DialogClose>
              <Button variant="destructive" disabled={confirm !== "WIPE"} onClick={wipe}>
                {t("settings.data.wipeConfirm")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
