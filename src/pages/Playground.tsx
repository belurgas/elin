import { useState } from "react";
import { api } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, PageShell, Textarea } from "../components/ui";

export function PlaygroundPage() {
  const { t } = useApp();
  const [code, setCode] = useState(t.playground.sample);
  const [output, setOutput] = useState("");
  const [busy, setBusy] = useState(false);

  async function run() {
    setBusy(true);
    try {
      setOutput(await api.eval(code));
    } catch (err) {
      setOutput(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <PageShell title={t.playground.title} subtitle={t.playground.subtitle}>
      <Card>
        <Textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          className="h-44 min-h-44"
        />
        <div className="mt-4">
          <Button disabled={busy} onClick={() => void run()}>
            {t.playground.run}
          </Button>
        </div>
      </Card>
      {output ? (
        <Card>
          <pre className="selectable whitespace-pre-wrap font-mono text-sm text-elixir-300">{output}</pre>
        </Card>
      ) : null}
    </PageShell>
  );
}
