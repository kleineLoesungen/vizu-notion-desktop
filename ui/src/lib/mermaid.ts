// Mermaid zeichnen.
//
// Zwei Wege hinein: ein ganzer Diagrammtext aus Rust (renderDiagram) und
// ```mermaid-Blöcke in gerendertem Markdown (renderDiagrams).
//
// Drei Dinge, die hier absichtlich so sind:
//
// 1. **Nachgeladen.** Mermaid ist mehrere Megabyte groß. `import("mermaid")`
//    lädt es erst, wenn eine Notiz tatsächlich ein Diagramm enthält — die
//    Anwendung startet ohne es. Vite legt es als eigenes Stück ins Bündel,
//    es kommt also trotzdem ohne Netz aus.
// 2. **securityLevel "strict".** Ein Diagramm darf keine Klick-Handler und
//    kein HTML in Beschriftungen mitbringen.
// 3. **Veraltete Aufträge werden verworfen.** Wer tippt, startet alle paar
//    hundert Millisekunden eine neue Darstellung. `isCurrent` verhindert, dass
//    eine langsame alte eine schnelle neue überschreibt.

type Mermaid = typeof import("mermaid")["default"];

let loading: Promise<Mermaid> | undefined;
let counter = 0;

function load(): Promise<Mermaid> {
  loading ??= import("mermaid").then((m) => m.default);
  return loading;
}

export const MERMAID_SELECTOR = "pre > code.language-mermaid";

/** Was beim Zeichnen herauskam. */
export type DiagramResult = { ok: true } | { ok: false; message: string };

/**
 * Zeichnet einen ganzen Diagrammtext in `host`.
 *
 * Der Text kommt aus `vizu_notion_core::template` — dieselbe Zeichenkette, die
 * auch `vizu-notion render` ausgibt. Hier wird nur gezeichnet.
 */
export async function renderDiagram(
  host: HTMLElement,
  text: string,
  options: { dark: boolean; isCurrent: () => boolean },
): Promise<DiagramResult> {
  const mermaid = await load();
  if (!options.isCurrent()) return { ok: true };

  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: options.dark ? "dark" : "default",
    fontFamily: getComputedStyle(host).fontFamily,
  });

  const id = `mermaid-${++counter}`;
  try {
    // parse wirft bei einem Syntaxfehler mit lesbarer Meldung. render allein
    // hinterlässt dabei ein Fehlerbild im <body>.
    await mermaid.parse(text);
    const { svg } = await mermaid.render(id, text);
    if (!options.isCurrent()) return { ok: true };
    host.innerHTML = svg;
    return { ok: true };
  } catch (error) {
    document.getElementById(`d${id}`)?.remove();
    if (options.isCurrent()) host.replaceChildren();
    return { ok: false, message: readable(error) };
  }
}

export async function renderDiagrams(
  root: HTMLElement,
  options: { dark: boolean; isCurrent: () => boolean },
): Promise<void> {
  const blocks = Array.from(root.querySelectorAll<HTMLElement>(MERMAID_SELECTOR));
  if (blocks.length === 0) return;

  const mermaid = await load();
  if (!options.isCurrent()) return;

  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: options.dark ? "dark" : "default",
    fontFamily: getComputedStyle(root).fontFamily,
  });

  for (const code of blocks) {
    const pre = code.parentElement;
    if (!pre) continue;
    const figure = document.createElement("figure");
    figure.className = "diagram";

    const id = `mermaid-${++counter}`;
    try {
      // parse wirft bei einem Syntaxfehler mit lesbarer Meldung. render
      // allein hinterlässt dabei ein Fehlerbild im <body>.
      await mermaid.parse(code.textContent ?? "");
      const { svg } = await mermaid.render(id, code.textContent ?? "");
      if (!options.isCurrent()) return;
      figure.innerHTML = svg;
    } catch (error) {
      if (!options.isCurrent()) return;
      figure.classList.add("diagram-error");
      figure.textContent = `Diagramm fehlerhaft: ${firstLine(error)}`;
      document.getElementById(`d${id}`)?.remove();
    }
    pre.replaceWith(figure);
  }
}

function firstLine(error: unknown): string {
  return readable(error).split("\n")[0] ?? "";
}

/**
 * Die Meldung von Mermaid, ganz.
 *
 * Bei einem Syntaxfehler steht in den Zeilen darunter die fehlerhafte Stelle
 * mit einem Zeiger — die erste Zeile allein („Parse error on line 2") sagt
 * niemandem, was falsch ist.
 */
function readable(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
