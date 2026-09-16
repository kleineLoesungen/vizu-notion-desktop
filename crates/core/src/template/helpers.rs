//! Die Helfer, die eine Vorlage benutzen kann.
//!
//! | Helfer | Wozu |
//! |---|---|
//! | `nodeId` | setzt der Umschreiber ein: Attribut → `nXXXXXX["Wert"]` |
//! | `group` | Zeilen nach einem Feld gruppieren |
//! | `group-item` | die Zeilen einer Gruppe durchlaufen |
//! | `palette` | eine von zehn unterscheidbaren Farben, nach Nummer |
//! | `join-rows` | zwei Quellen über ein Feld zusammenführen |
//! | `lookup-by` | passende Zeilen einer anderen Quelle, verschachtelt |
//!
//! Die Ausgabe muss der von handlebars.js entsprechen — geprüft an den
//! Referenzfällen in `crates/core/tests/fixtures/templates/`.

use std::sync::{Arc, Mutex};

use handlebars::{
    BlockContext, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, Renderable, ScopedJson,
};
use serde_json::{Map, Value, json};

use super::style;

/// Zehn gut unterscheidbare Farben (Tableau 10), wie in der Webapp.
const PALETTE: [&str; 10] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

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
    hb.register_helper("group", Box::new(Group));
    hb.register_helper("group-item", Box::new(GroupItem));
    hb.register_helper("palette", Box::new(Palette));
    hb.register_helper("join-rows", Box::new(JoinRows));
    hb.register_helper("lookup-by", Box::new(LookupBy));
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
