import { useEffect, useState } from "react";
import { pickFolder } from "../../lib/api";
import { Button, Checkbox, Chip, Field, Input, Modal } from "../../components/ui";
import type { Kit } from "../../types";
import type { Labels } from "./types";

const templates = [
  ["mix", "mix"],
  ["mix-sup", "mixSup"],
  ["phoenix", "phoenix"],
  ["phoenix-live", "live"],
] as const;

export function CreateDialog({
  open,
  t,
  host,
  catalog,
  busy,
  onClose,
  onCreate,
}: {
  open: boolean;
  t: Labels;
  host: string;
  catalog: Kit[];
  busy: boolean;
  onClose: () => void;
  onCreate: (name: string, directory: string, template: string, kits: string[]) => void;
}) {
  const [name, setName] = useState("hello_elin");
  const [directory, setDirectory] = useState(host);
  const [template, setTemplate] = useState("mix");
  const phoenix = template.startsWith("phoenix");
  const [kitIds, setKitIds] = useState<string[]>(() =>
    catalog.filter((k) => k.defaultOn && (!k.phoenixOnly || phoenix)).map((k) => k.id),
  );

  useEffect(() => {
    if (open) setDirectory((current) => current || host);
  }, [open, host]);

  useEffect(() => {
    setKitIds(catalog.filter((k) => k.defaultOn && (!k.phoenixOnly || phoenix)).map((k) => k.id));
  }, [catalog, phoenix]);

  return (
    <Modal
      open={open}
      onClose={onClose}
      dismissible={!busy}
      title={t.createTitle}
      footer={
        <>
          <Button variant="ghost" disabled={busy} onClick={onClose}>
            {t.close}
          </Button>
          <Button disabled={busy} onClick={() => onCreate(name, directory, template, kitIds)}>
            {t.create}
          </Button>
        </>
      }
    >
      <div className="grid gap-4">
        <Field label={t.name}>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label={t.folder}>
          <div className="flex gap-2">
            <Input className="flex-1" value={directory} onChange={(e) => setDirectory(e.target.value)} />
            <Button
              variant="ghost"
              onClick={async () => {
                const folder = await pickFolder();
                if (folder) setDirectory(folder);
              }}
            >
              {t.browse}
            </Button>
          </div>
        </Field>
        <div className="flex flex-wrap gap-2">
          {templates.map(([id, key]) => (
            <Chip key={id} active={template === id} onClick={() => setTemplate(id)}>
              {t[key]}
            </Chip>
          ))}
        </div>
        <p className="text-xs text-mist-300">{t.kitsHint}</p>
        <div className="grid gap-2">
          {catalog
            .filter((k) => !k.phoenixOnly || phoenix)
            .map((kit) => (
              <Checkbox
                key={kit.id}
                checked={kitIds.includes(kit.id)}
                onChange={(on) => setKitIds(on ? [...kitIds, kit.id] : kitIds.filter((id) => id !== kit.id))}
              >
                {kit.name}
                <span className="ml-2 text-mist-300">{kit.summary}</span>
              </Checkbox>
            ))}
        </div>
      </div>
    </Modal>
  );
}
