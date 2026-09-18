// Hilfe beim Schreiben einer Vorlage: die Felder einer Quelle zum Anklicken
// und Bausteine für wiederkehrende Muster.
//
// Zeichnet nur. Was eingefügt wird, meldet `onInsert` — an der Schreibmarke
// setzt es der Editor ein. Die Bausteine kommen aus core
// (`template::examples::blocks`), wo sie mit echten Daten geprüft werden.

import { useState } from "react";
import type { Block } from "../bindings";

/** Eine Quelle, wie der Editor sie braucht: Name und Rollen. */
export type SourceFields = { name: string; roles: string[] };

type Props = {
  sources: SourceFields[];
  blocks: Block[];
  /** Der Vorlagentext — um die Quelle vorzuwählen, die er schon nennt. */
  body: string;
  /** Ein Feld: einfach an die Schreibmarke. */
  onInsert: (text: string) => void;
  /**
   * Ein Baustein: über core, damit Kopf, Diagrammart und `sources` stimmen
   * (`template::assemble`). `sources` sind die Quellen, die er benutzt.
   */
  onInsertBlock: (snippet: string, sources: string[]) => void;
};

export function TemplateHelper({ sources, blocks, body, onInsert, onInsertBlock }: Props) {
  // Vorgewählt ist die erste Quelle, die im Text schon vorkommt: Wer an
  // „Projekte" schreibt, will die Felder von „Projekte" sehen.
  const named = sources.find((s) => body.includes(s.name)) ?? sources[0];
  const [chosen, setChosen] = useState<string | null>(null);
  const source = sources.find((s) => s.name === chosen) ?? named;

  const [asText, setAsText] = useState(false);
  const [blockName, setBlockName] = useState("");
  const [field, setField] = useState("");
  const [second, setSecond] = useState("");

  if (!source) {
    return (
      <p className="muted">
        Noch keine Quelle eingerichtet — ohne Quelle gibt es keine Felder zum Einfügen.
      </p>
    );
  }

  const block = blocks.find((b) => b.name === blockName);
  const fieldValue = field || source.roles.find((r) => r !== "title") || source.roles[0] || "";
  const others = sources.filter((s) => s.name !== source.name);
  const secondValue = second || others[0]?.name || source.name;

  function insertBlock() {
    if (!block || !source) return;
    onInsertBlock(
      block.body
        .replaceAll("QUELLE", source.name)
        .replaceAll("ZWEITE", secondValue)
        .replaceAll("FELD", fieldValue),
      block.needs_second ? [source.name, secondValue] : [source.name],
    );
  }

  return (
    <details className="template-helper" open>
      <summary>Felder und Bausteine</summary>

      {/* Zwei Spalten: links, worum es geht; rechts, was man tun kann. Jede
          Zeile steht an derselben Stelle, egal wie breit der Editor ist. */}
      <div className="helper-grid">
        <span className="helper-caption">Felder</span>
        <div className="helper-cell">
          <div className="helper-line">
            <select
              aria-label="Quelle"
              value={source.name}
              onChange={(e) => setChosen(e.target.value)}
            >
              {sources.map((s) => (
                <option key={s.name} value={s.name}>
                  {s.name}
                </option>
              ))}
            </select>
            <fieldset className="segmented" aria-label="Einfügen als">
              <button type="button" aria-pressed={!asText} onClick={() => setAsText(false)}>
                Knoten
              </button>
              <button type="button" aria-pressed={asText} onClick={() => setAsText(true)}>
                Text
              </button>
            </fieldset>
          </div>

          {/* Ein Klick fügt das Feld an der Schreibmarke ein: als Knoten
              ({{feld}}) oder als roher Text ({{this.feld}}) — so steht es
              auch im Spickzettel. Die Seiten-ID ist immer Text. */}
          <fieldset className="field-chips" aria-label={`Felder von ${source.name}`}>
            {source.roles.map((role) => (
              <button
                key={role}
                type="button"
                className="chip"
                onClick={() => onInsert(asText ? `{{this.${role}}}` : `{{${role}}}`)}
              >
                {role}
              </button>
            ))}
            <button type="button" className="chip" onClick={() => onInsert("{{this.id}}")}>
              id
            </button>
            <button
              type="button"
              className="chip"
              onClick={() =>
                onInsertBlock(`{{#each ${source.name}}}\n  \n{{/each}}\n`, [source.name])
              }
            >
              #each {source.name}
            </button>
          </fieldset>
        </div>

        <span className="helper-caption">Baustein</span>
        <div className="helper-cell">
          <select
            aria-label="Baustein"
            value={blockName}
            onChange={(e) => setBlockName(e.target.value)}
          >
            <option value="">auswählen …</option>
            {blocks.map((b) => (
              <option key={b.name} value={b.name}>
                {b.name}
              </option>
            ))}
          </select>

          {block && (
            <>
              <p className="muted helper-purpose">{block.purpose}</p>
              <div className="helper-line">
                {block.needs_field && (
                  <label className="inline-label">
                    Feld
                    <select value={fieldValue} onChange={(e) => setField(e.target.value)}>
                      {source.roles.map((role) => (
                        <option key={role} value={role}>
                          {role}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                {block.needs_second && (
                  <label className="inline-label">
                    Zweite Quelle
                    <select value={secondValue} onChange={(e) => setSecond(e.target.value)}>
                      {(others.length > 0 ? others : [source]).map((s) => (
                        <option key={s.name} value={s.name}>
                          {s.name}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                <button type="button" className="insert-button" onClick={insertBlock}>
                  Einfügen
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </details>
  );
}
