# Translating Elin

Elin ships English and Russian. Other languages come from people who use the app and want it in their own words.

You do not need to know Rust. You do not need to redesign anything. You copy one file, translate the sentences, and open a pull request.

## What you are translating

Every visible string lives under `src/i18n/`.

| File | What it is |
| --- | --- |
| `en.ts` | English. This is the source of truth. New UI text is added here first. |
| `ru.ts` | Russian. Same keys as English, different sentences. |
| `index.ts` | The list of languages the Settings page shows. |

Keys stay in English (`home.title`, `workspace.run`). You only change the **values** — the words a person reads.

## Add a language

1. Fork the repo on GitHub and clone your fork.
2. Create a branch. Name it after the language, for example `i18n/de` or `i18n/pt-BR`.
3. Copy the English file:

```bash
cp src/i18n/en.ts src/i18n/de.ts
```

Use the usual short code: `de`, `fr`, `es`, `pt`, `ja`, `zh`… If you need a region, `pt-BR` is fine as a filename, but the `id` in the next step must be a simple code the app can store (`pt` or `br` — pick one and stay consistent).

4. Open the new file. Change `export const en` to `export const de` (or whatever the code is). Delete `export type Dictionary = typeof en` if you copied it — that line belongs only in `en.ts`.
5. Translate every quoted string. Leave `{…}` interpolations and words like `mix.exs`, `PATH`, `OTP`, `Credo` as they are unless your language has a settled equivalent.
6. Register it in `src/i18n/index.ts`:

```ts
import { de } from "./de";

export const locales = [
  { id: "en", native: "English", english: "English" },
  { id: "ru", native: "Русский", english: "Russian" },
  { id: "de", native: "Deutsch", english: "German" },
] as const;

export const dictionaries: Record<Locale, Dictionary> = { en, ru, de };
```

`native` is what speakers call the language. `english` is the English name, shown as a hint in the Settings dropdown so people browsing still know what they clicked.

That `locales` array **is** the dropdown on the Settings page. A new row there is how the language appears in the app. You do not add a second control anywhere else.

7. Make TypeScript happy:

```bash
npx tsc --noEmit
```

If a key is missing or extra, the compiler points at the line. That is the whole review for structure.

8. Run the app (`npm run tauri dev`), open **Settings**, open the language dropdown, pick your language. Check the home page, Settings itself, and a Studio window if you can.

## Open a pull request

```bash
git add src/i18n
git commit -m "Add German translation"
git push -u origin HEAD
```

Then open a PR against `master` (or `main`). Keep the PR to translation files plus the two lines in `index.ts`. Do not mix in refactors, formatter noise, or unrelated bugfixes — those belong in a separate PR.

In the PR body, say:

- which language;
- that you ran `npx tsc --noEmit`;
- anything you left in English on purpose (product names, commands).

## Keep a translation up to date

English grows when the app grows. A translation that was complete last month may be missing three new keys.

```bash
git fetch origin
git checkout your-branch
git merge origin/master
```

If Git reports a conflict in `en.ts`, take the incoming English keys, then copy those new keys into your language file and translate them. `tsc` will list anything still missing.

Prefer **merge** over rebase if you are not used to rebase. Either is fine; merge is harder to get stuck in.

After the merge:

```bash
npx tsc --noEmit
```

Push the branch. The existing PR updates itself.

## What not to do

- Do not translate keys. `workspace.run` stays `workspace.run`.
- Do not wrap the file in a different shape (JSON, YAML, nested differently). The app reads this TypeScript object.
- Do not “improve” English in `en.ts` inside a translation PR. Open a separate issue or PR for wording.
- Do not add a language by editing the old single-file dump — that file is gone. One language, one file.

## Where the user picks it

**Settings** in the main window — a dropdown, not a grid of cards. The `locales` list in `index.ts` is that dropdown. The choice is stored on this PC and broadcast to Studio, the tray, and toasts immediately. There is no language toggle in the titlebar.
