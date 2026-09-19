//! Die Helfer, die eine Vorlage benutzen kann.
//!
//! | Helfer | Wozu |
//! |---|---|
//! | `nodeId` | setzt der Umschreiber ein: Attribut → `nXXXXXX["Wert"]` |
//! | `classId` | dieselbe Kennung **ohne** Kasten — für `classDef`-Zeilen |
//! | `group` | Zeilen nach einem Feld gruppieren |
//! | `group-item` | die Zeilen einer Gruppe durchlaufen |
//! | `palette` | eine von zehn unterscheidbaren Farben, nach Nummer |
//! | `join-rows` | zwei Quellen über ein Feld zusammenführen |
//! | `lookup-by` | passende Zeilen einer anderen Quelle, verschachtelt |
//!
//! Die Ausgabe muss der von handlebars.js entsprechen — geprüft an den
//! Referenzfällen in `crates/core/tests/fixtures/templates/`.
//!
//! Dazu kommen Helfer, die die Webapp nicht kannte. Sie ändern keine
//! vorhandene Vorlage — die benutzt sie ja nicht —, aber der Assistent baut
//! damit Vorlagen, die nicht an Sonderzeichen oder Gruppen scheitern:
//!
//! | Helfer | Wozu |
//! |---|---|
//! | `node` | ein Knoten **ohne** Gruppe in der Kennung — damit Pfeile auch Knoten in Rahmen treffen |
//! | `valueClass` | eine Klasse je Wert, gleich in jeder Gruppe — für Farbe je Wert |
//! | `label` | Freitext, der in Gantt, Mindmap und Kreisdiagramm nichts zerbricht |
//! | `day` | nur der Tag aus einem Notion-Datum, ohne Uhrzeit |
//! | `sum` | Summe eines Zahlenfelds über die Seiten einer Gruppe |
//! | `unique` | jede Seite einmal — ohne die Kopien je Relationsziel |
//! | `group-pages` | wie `group`, aber jede Seite je Gruppe einmal |

use std::sync::{Arc, Mutex};

use handlebars::{
    BlockContext, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, Renderable, ScopedJson,
};
use serde_json::{Map, Value, json};

use super::style;

use crate::palette::TABLEAU_10 as PALETTE;

/// Knoten, die eine CSS-Klasse bekommen sollen: Kennung → Klassenname.
///
/// Wird beim Zeichnen gefüllt und danach zu `class …`-Zeilen verarbeitet.
pub type ClassAssignments = Arc<Mutex<Vec<(String, String)>>>;

/// Eine stabile Knotenkennung aus einem Wert.
///
/// FNV-1a über **UTF-16-Codeeinheiten**, nicht über UTF-8-Bytes: `charCodeAt`
/// in JavaScript zählt so, und nur damit ergeben Emoji dieselbe Kennung wie in
/// der Webapp. Ausgabe: `n` + Basis 36, auf sechs Stellen aufgefüllt —
/// Mermaid verträgt keine Kennung, die mit einer Ziffer beginnt.
pub fn stable_id(value: &str, scope: &str) -> String {
    let input = if scope.is_empty() {
        value.to_string()
    } else {
        format!("{scope}\0{value}")
    };
    let mut hash: u32 = 0x811c_9dc5;
    for unit in input.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("n{:0>6}", base36(hash))
}

fn base36(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).expect("nur ASCII")
}

/// `String(x ?? '')` aus JavaScript.
pub fn js_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| js_string(Some(v)))
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// Maskierung wie `Handlebars.escapeExpression` — handlebars-rust maskiert von
/// sich aus andere Zeichen.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '`' => out.push_str("&#x60;"),
            '=' => out.push_str("&#x3D;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn register(hb: &mut Handlebars<'_>, classes: ClassAssignments) {
    hb.register_escape_fn(escape);
    hb.register_helper("nodeId", Box::new(NodeId(classes)));
    hb.register_helper("classId", Box::new(ClassId));
    hb.register_helper("group", Box::new(Group));
    hb.register_helper("group-item", Box::new(GroupItem));
    hb.register_helper("palette", Box::new(Palette));
    hb.register_helper("join-rows", Box::new(JoinRows));
    hb.register_helper("lookup-by", Box::new(LookupBy));
    hb.register_helper("node", Box::new(NodeHelper));
    hb.register_helper("valueClass", Box::new(ValueClass));
    hb.register_helper("label", Box::new(Label));
    hb.register_helper("day", Box::new(Day));
    hb.register_helper("sum", Box::new(Sum));
    hb.register_helper("unique", Box::new(Unique));
    hb.register_helper("group-pages", Box::new(GroupPages));
}

/// Zeilen ohne Doppel: je Seiten-ID die erste.
///
/// Eine Seite mit drei Zielen steht als drei Zeilen im Kontext — richtig für
/// Pfeile (einer je Ziel), falsch für einen Balken je Seite oder die Anzahl
/// der Seiten. So war es in der Webapp, und `group` bleibt deshalb dabei.
fn unique_rows(rows: &[Value]) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    rows.iter()
        .filter(|row| seen.insert(js_string(row.get("id"))))
        .cloned()
        .collect()
}

/// `(unique Quelle)` — jede Seite einmal.
struct Unique;

impl HelperDef for Unique {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let rows = h
            .param(0)
            .and_then(|p| p.value().as_array())
            .map(|rows| unique_rows(rows))
            .unwrap_or_default();
        Ok(ScopedJson::Derived(Value::Array(rows)))
    }
}

/// `(group-pages Quelle "feld")` — wie `group`, aber jede Seite je Gruppe
/// einmal. Eine Seite mit zwei Zielen steht so unter jedem ihrer Ziele — aber
/// nicht dreimal unter demselben Status, nur weil sie drei Nachfolger hat.
struct GroupPages;

impl HelperDef for GroupPages {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let grouped = Group.call_inner(h, r, ctx, rc)?;
        let mut groups = grouped.as_json().clone();
        for group in groups.as_array_mut().into_iter().flatten() {
            if let Some(Value::Array(items)) = group.get_mut("items") {
                *items = unique_rows(items);
            }
        }
        Ok(ScopedJson::Derived(groups))
    }
}

/// `{{node wert "Quelle"}}` — wie der Knoten, den der Umschreiber aus
/// `{{feld}}` macht, aber **ohne** den Gruppenschlüssel in der Kennung.
///
/// `nodeId` nimmt in einer Gruppe deren Schlüssel mit in die Kennung, damit
/// eine Seite in zwei Gruppen zweimal stehen kann. Das hat einen Preis: Ein
/// Pfeil von außen trifft diesen Knoten nicht. Mit `node` stimmt die Kennung
/// mit der aus `{{title}}` außerhalb der Gruppe überein.
struct NodeHelper;

impl HelperDef for NodeHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = js_string(h.param(0).map(|p| p.value()));
        let source = js_string(h.param(1).map(|p| p.value()));
        let id = stable_id(&value, &source);
        let label: String = value.chars().filter(|c| !"\"[]{}()".contains(*c)).collect();
        let (open, close) = style::brackets("rectangle");
        out.write(&format!("{id}{open}{label}{close}"))?;
        Ok(())
    }
}

/// `{{valueClass wert}}` — ein Klassenname, der nur vom Wert abhängt.
///
/// `classId` rechnet wie `nodeId` den Gruppenschlüssel mit ein; damit hätte
/// derselbe Status in der `classDef`-Schleife eine andere Klasse als am
/// Knoten. Hier nicht.
struct ValueClass;

impl HelperDef for ValueClass {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = js_string(h.param(0).map(|p| p.value()));
        out.write(&format!("v-{}", stable_id(&value, "value")))?;
        Ok(())
    }
}

/// `{{label wert "Ersatz"}}` — Freitext für zeilenbasierte Diagrammarten.
///
/// Gantt, Mindmap und Kreisdiagramm lesen Doppelpunkte, Klammern,
/// Anführungszeichen und `#` als Syntax. Die fallen weg; ist danach nichts
/// übrig, steht der Ersatz da. Ohne Maskierung, denn `&amp;` stünde sonst
/// wörtlich im Diagramm.
struct Label;

impl HelperDef for Label {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let text = label_text(&js_string(h.param(0).map(|p| p.value())));
        let text = if text.is_empty() {
            label_text(&js_string(h.param(1).map(|p| p.value())))
        } else {
            text
        };
        out.write(&text)?;
        Ok(())
    }
}

/// Was [`Label`] aus einem Text macht.
pub fn label_text(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || ":;#\"'`[](){}<>|\\".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    // `%%` leitet in Mermaid einen Kommentar ein.
    cleaned
        .replace("%%", "%")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `{{day datum}}` — `2026-04-01T09:30:00+02:00` → `2026-04-01`.
///
/// Gantt erwartet ein Datum im Format von `dateFormat`; mit Uhrzeit scheitert
/// die ganze Zeile. Ist es kein Datum, bleibt die Ausgabe leer.
struct Day;

impl HelperDef for Day {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let raw = js_string(h.param(0).map(|p| p.value()));
        let day = raw.get(..10).unwrap_or_default();
        let bytes = day.as_bytes();
        let looks_like_day = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
        if looks_like_day {
            out.write(day)?;
        }
        Ok(())
    }
}

/// `{{sum items "feld"}}` — Summe eines Zahlenfelds über die Seiten einer
/// Gruppe, jede Seite einmal. Was keine Zahl ist, zählt nicht mit.
struct Sum;

impl HelperDef for Sum {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let field = js_string(h.param(1).map(|p| p.value()));
        // Je Seite einmal — sonst zählten die Kopien je Relationsziel mit.
        let rows = h
            .param(0)
            .and_then(|p| p.value().as_array())
            .map(|rows| unique_rows(rows))
            .unwrap_or_default();
        let total: f64 = rows
            .iter()
            .filter_map(|row| js_string(row.get(&field)).trim().parse::<f64>().ok())
            .sum();
        // Wie JavaScript: `12` statt `12.0`.
        let value = if total.fract() == 0.0 && total.abs() < 9.0e15 {
            json!(total as i64)
        } else {
            json!(total)
        };
        Ok(ScopedJson::Derived(value))
    }
}

/// `{{nodeId "attribut" wert shape=… className=… source=…}}`
struct NodeId(ClassAssignments);

impl HelperDef for NodeId {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value = js_string(h.param(1).map(|p| p.value()));
        let source = hash(h, "source");
        // Innerhalb einer Gruppe gehört der Gruppenschlüssel zur Kennung.
        let group_key = rc
            .evaluate(ctx, "_groupKey")
            .ok()
            .map(|v| js_string(Some(v.as_json())))
            .unwrap_or_default();
        let scope = if group_key.is_empty() {
            source
        } else {
            format!("{source}\0{group_key}")
        };

        let id = stable_id(&value, &scope);
        // Klammern gehören zur Mermaid-Syntax und dürfen nicht im Text stehen.
        let label: String = value.chars().filter(|c| !"\"[]{}()".contains(*c)).collect();
        let shape = h
            .hash_get("shape")
            .map(|v| js_string(Some(v.value())))
            .unwrap_or_else(|| "rectangle".to_string());
        let (open, close) = style::brackets(&shape);

        if let Some(class) = h.hash_get("className") {
            let class = js_string(Some(class.value()));
            let mut assignments = self.0.lock().unwrap_or_else(|e| e.into_inner());
            match assignments.iter_mut().find(|(node, _)| *node == id) {
                Some(entry) => entry.1 = class,
                None => assignments.push((id.clone(), class)),
            }
        }
        out.write(&format!("{id}{open}{label}{close}"))?;
        Ok(())
    }
}

/// `{{classId "attribut" wert}}` — nur die Kennung, ohne Kasten und Text.
///
/// Gibt es in der Webapp nicht. Dort empfiehlt `config/mermaid.example`
/// `classDef cls-{{nodeId …}}`, was zu `classDef cls-nXXXXXX["Done"]` wird —
/// einem Klassennamen mit Klammern, den Mermaid verwirft. Mit `classId` wird
/// daraus `classDef cls-nXXXXXX`. Vorhandene Vorlagen ändert der Helfer nicht,
/// weil der Umschreiber ihn nie von sich aus einsetzt.
struct ClassId;

impl HelperDef for ClassId {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Dieselbe Rechnung wie nodeId, damit die Klasse zum Knoten passt.
        let value = js_string(h.param(1).map(|p| p.value()));
        let source = hash(h, "source");
        let group_key = rc
            .evaluate(ctx, "_groupKey")
            .ok()
            .map(|v| js_string(Some(v.as_json())))
            .unwrap_or_default();
        let scope = if group_key.is_empty() {
            source
        } else {
            format!("{source}\0{group_key}")
        };
        out.write(&stable_id(&value, &scope))?;
        Ok(())
    }
}

fn hash(h: &Helper<'_>, key: &str) -> String {
    h.hash_get(key)
        .map(|v| js_string(Some(v.value())))
        .unwrap_or_default()
}

/// `(group Zeilen "feld")` → `[{ feld: Wert, _groupKey: Wert, items: [...] }]`
struct Group;

impl HelperDef for Group {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let (Some(Value::Array(rows)), Some(field)) = (
            h.param(0).map(|p| p.value()),
            h.param(1).and_then(|p| p.value().as_str()),
        ) else {
            return Ok(ScopedJson::Derived(json!([])));
        };

        let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
        for row in rows {
            let key = js_string(row.get(field));
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some(group) => group.1.push(row.clone()),
                None => groups.push((key, vec![row.clone()])),
            }
        }
        let out: Vec<Value> = groups
            .into_iter()
            .map(|(key, items)| {
                let mut object = Map::new();
                object.insert(field.to_string(), Value::String(key.clone()));
                object.insert("_groupKey".to_string(), Value::String(key));
                object.insert("items".to_string(), Value::Array(items));
                Value::Object(object)
            })
            .collect();
        Ok(ScopedJson::Derived(Value::Array(out)))
    }
}

/// `{{#group-item}} … {{/group-item}}` innerhalb von `{{#each (group …)}}`.
struct GroupItem;

impl HelperDef for GroupItem {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let items = rc.evaluate(ctx, "items")?.as_json().clone();
        let key = js_string(Some(rc.evaluate(ctx, "_groupKey")?.as_json()));
        let Some(block) = h.template() else {
            return Ok(());
        };
        for item in items.as_array().into_iter().flatten() {
            let mut object = item.as_object().cloned().unwrap_or_default();
            // Der Schlüssel wandert mit: die Kennung eines Knotens innerhalb
            // einer Gruppe hängt an ihm.
            object.insert("_groupKey".to_string(), Value::String(key.clone()));
            let mut scope = BlockContext::new();
            scope.set_base_value(Value::Object(object));
            rc.push_block(scope);
            let result = block.render(r, ctx, rc, out);
            rc.pop_block();
            result?;
        }
        Ok(())
    }
}

/// `{{palette @index}}`
struct Palette;

impl HelperDef for Palette {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let index = h
            .param(0)
            .and_then(|p| p.value().as_f64())
            .unwrap_or(0.0)
            .abs() as usize;
        Ok(ScopedJson::Derived(json!(PALETTE[index % PALETTE.len()])))
    }
}

/// `(join-rows A "feldA" B "feldB" "praefix")` — linker Verbund über ein Feld.
struct JoinRows;

impl HelperDef for JoinRows {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let param = |i: usize| h.param(i).map(|p| p.value().clone());
        let (
            Some(Value::Array(left)),
            Some(Value::String(left_field)),
            Some(Value::Array(right)),
            Some(Value::String(right_field)),
        ) = (param(0), param(1), param(2), param(3))
        else {
            return Ok(ScopedJson::Derived(json!([])));
        };
        let prefix = match param(4) {
            Some(Value::String(p)) if !p.is_empty() => p,
            _ => "b".to_string(),
        };

        let mut result = Vec::new();
        for row in &left {
            let key = js_string(row.get(&left_field));
            let matches: Vec<&Value> = right
                .iter()
                .filter(|other| js_string(other.get(&right_field)) == key)
                .collect();
            if matches.is_empty() {
                // Linker Verbund: Zeilen ohne Partner bleiben, wie sie sind.
                result.push(row.clone());
                continue;
            }
            for other in matches {
                let mut merged = row.as_object().cloned().unwrap_or_default();
                for (k, v) in other.as_object().into_iter().flatten() {
                    merged.insert(format!("{prefix}_{k}"), v.clone());
                }
                result.push(Value::Object(merged));
            }
        }
        Ok(ScopedJson::Derived(Value::Array(result)))
    }
}

/// `(lookup-by Zeilen "feld" wert)` — passende Zeilen für einen inneren Block.
struct LookupBy;

impl HelperDef for LookupBy {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let (Some(Value::Array(rows)), Some(Value::String(field))) =
            (h.param(0).map(|p| p.value()), h.param(1).map(|p| p.value()))
        else {
            return Ok(ScopedJson::Derived(json!([])));
        };
        let wanted = js_string(h.param(2).map(|p| p.value()));
        let found: Vec<Value> = rows
            .iter()
            .filter(|row| js_string(row.get(field)) == wanted)
            .cloned()
            .collect();
        Ok(ScopedJson::Derived(Value::Array(found)))
    }
}
