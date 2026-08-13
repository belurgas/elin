import { browse } from "../lib/api";
import { useApp } from "../state";
import { PageShell } from "../components/ui";

const links = [
  {
    title: "Getting started",
    body: "Official language tour. Read this before any framework.",
    url: "https://elixir-lang.org/getting-started/introduction.html",
  },
  {
    title: "Install notes",
    body: "The page Elin is built to replace — still useful as the source of truth.",
    url: "https://elixir-lang.org/install.html",
  },
  {
    title: "Elixir School",
    body: "Friendly lessons from basics to OTP and Phoenix.",
    url: "https://elixirschool.com/",
  },
  {
    title: "Exercism Elixir track",
    body: "Small exercises with mentoring. Perfect after `mix new`.",
    url: "https://exercism.org/tracks/elixir",
  },
  {
    title: "HexDocs",
    body: "Every package’s docs, including Elixir itself.",
    url: "https://hexdocs.pm/elixir",
  },
  {
    title: "Phoenix",
    body: "When you want a web app. Elin’s Projects page can scaffold it.",
    url: "https://www.phoenixframework.org/",
  },
  {
    title: "Livebook",
    body: "Interactive notebooks in Elixir. Magical for learning.",
    url: "https://livebook.dev/",
  },
  {
    title: "Elixir Forum",
    body: "Kind, slow, and precise. The best place to ask a real question.",
    url: "https://elixirforum.com/",
  },
];

export function LearnPage() {
  const { t } = useApp();
  return (
    <PageShell title={t.learn.title} subtitle={t.learn.subtitle}>
      <div className="surface divide-y divide-white/6 overflow-hidden rounded-xl">
        {links.map((link) => (
          <button
            type="button"
            key={link.url}
            onClick={() => void browse(link.url)}
            className="flex w-full items-baseline gap-4 px-4 py-2.5 text-left hover:bg-white/4"
          >
            <span className="w-40 shrink-0 text-[13px] font-medium">{link.title}</span>
            <span className="min-w-0 truncate text-[13px] text-mist-300">{link.body}</span>
          </button>
        ))}
      </div>
    </PageShell>
  );
}
