import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
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
  const qc = useQueryClient();
  const [confirm, setConfirm] = useState("");
  const [open, setOpen] = useState(false);

  const wipe = async () => {
    try {
      await api.wipe();
      toast.success("Local data wiped");
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
        <CardTitle>Data</CardTitle>
        <CardDescription>
          Permanently delete all agents, skills, conversations, embeddings, and stored API keys.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger asChild>
            <Button variant="destructive">Wipe local data</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Wipe local data?</DialogTitle>
              <DialogDescription>
                This cannot be undone. Type <code className="font-mono">WIPE</code> to confirm.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-1">
              <Label htmlFor="confirm">Confirmation</Label>
              <Input id="confirm" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button variant="outline">Cancel</Button>
              </DialogClose>
              <Button variant="destructive" disabled={confirm !== "WIPE"} onClick={wipe}>
                Wipe
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
