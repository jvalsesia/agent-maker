import { useEffect, useState } from "react";
import { toast } from "sonner";
import { ApiError } from "@/lib/api";
import { useSettings, useUpdateSettings } from "@/hooks/useSettings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function MemoryDefaultsSection() {
  const { data } = useSettings();
  const update = useUpdateSettings();
  const [recentN, setRecentN] = useState(10);
  const [topK, setTopK] = useState(5);

  useEffect(() => {
    if (data) {
      setRecentN(data.memory_defaults.recent_n);
      setTopK(data.memory_defaults.top_k);
    }
  }, [data]);

  const save = async () => {
    try {
      await update.mutateAsync({ memory_defaults: { recent_n: recentN, top_k: topK } });
      toast.success("Memory defaults saved");
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Memory defaults</CardTitle>
        <CardDescription>
          Recent N: verbatim turns included in every request. Top K: older turns retrieved via semantic search.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4 max-w-md">
          <div className="space-y-1">
            <Label htmlFor="recentN">Recent N (4–30)</Label>
            <Input
              id="recentN"
              type="number"
              min={4}
              max={30}
              value={recentN}
              onChange={(e) => setRecentN(Number(e.target.value))}
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="topK">Top K (0–10)</Label>
            <Input
              id="topK"
              type="number"
              min={0}
              max={10}
              value={topK}
              onChange={(e) => setTopK(Number(e.target.value))}
            />
          </div>
        </div>
        <Button onClick={save} disabled={update.isPending}>Save</Button>
      </CardContent>
    </Card>
  );
}
