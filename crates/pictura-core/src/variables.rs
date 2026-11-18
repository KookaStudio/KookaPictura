use crate::layer_comp::{ensure_layer_ids, layer_id, layer_ids, LayerId};
use crate::{layer_pixel_locked, Document, Layer, LayerKind, LayerStack};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableKind {
    Visibility,
    Text,
    Pixels,
}

impl VariableKind {
    pub fn accepts(self, layer: &Layer) -> bool {
        !layer.background
            && match self {
                Self::Visibility => true,
                Self::Text => {
                    !layer.lock.is_all()
                        && layer.type_tool().is_some_and(|tool| tool.style.is_some())
                }
                Self::Pixels => {
                    !layer_pixel_locked(layer)
                        && !layer.pixel_data_irrelevant
                        && layer.kind() == LayerKind::Pixel
                }
            }
    }

    pub fn accepts_value(self, value: &Value) -> bool {
        matches!(
            (self, value),
            (Self::Visibility, Value::Visibility(_))
                | (Self::Text, Value::Text(_))
                | (Self::Pixels, Value::File(_) | Value::KeepPixels)
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleMethod {
    #[default]
    Fit,
    Fill,
    AsIs,
    Conform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub method: ScaleMethod,
    pub horizontal: u8,
    pub vertical: u8,
    pub clip: bool,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            method: ScaleMethod::Fit,
            horizontal: 1,
            vertical: 1,
            clip: false,
        }
    }
}

impl Placement {
    pub fn valid(self) -> bool {
        self.horizontal <= 2
            && self.vertical <= 2
            && (!self.clip || matches!(self.method, ScaleMethod::Fill | ScaleMethod::AsIs))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Visibility(bool),
    Text(String),
    File(String),
    KeepPixels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub layer_id: LayerId,
    pub placement: Placement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub id: u64,
    pub kind: VariableKind,
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataSet {
    pub name: String,
    pub values: BTreeMap<u64, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Template {
    definitions: BTreeMap<String, Variable>,
    sets: Vec<DataSet>,
    issues: Vec<String>,
    protected_ids: BTreeMap<LayerId, usize>,
}

#[derive(Clone, Debug, Default)]
pub struct VariableStore {
    data: Arc<Template>,
    dirty: bool,
}

impl PartialEq for VariableStore {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for VariableStore {}

pub fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':'))
        && bytes
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.' | b'-'))
}

pub fn valid_xml_text(text: &str) -> bool {
    text.chars().all(|c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
}

pub fn default_set_name(mut used: impl FnMut(&str) -> bool) -> String {
    for index in 1.. {
        let name = format!("Data Set {index}");
        if !used(&name) {
            return name;
        }
    }
    unreachable!()
}

impl VariableStore {
    pub fn from_file(
        definitions: BTreeMap<String, Variable>,
        sets: Vec<DataSet>,
        issues: Vec<String>,
        protected_ids: BTreeMap<LayerId, usize>,
    ) -> Self {
        Self {
            data: Arc::new(Template {
                definitions,
                sets,
                issues,
                protected_ids,
            }),
            dirty: false,
        }
    }

    pub fn definitions(&self) -> &BTreeMap<String, Variable> {
        &self.data.definitions
    }

    pub fn sets(&self) -> &[DataSet] {
        &self.data.sets
    }

    pub fn issues(&self) -> &[String] {
        &self.data.issues
    }

    pub fn protected_ids(&self) -> &BTreeMap<LayerId, usize> {
        &self.data.protected_ids
    }

    pub fn preserves_identities(&self, layers: &LayerStack) -> bool {
        if self.protected_ids().is_empty() {
            return true;
        }
        let mut counts = BTreeMap::new();
        count_protected_ids(layers, self.protected_ids(), &mut counts);
        &counts == self.protected_ids()
    }

    pub fn protects_layer(&self, layer: &Layer) -> bool {
        !self.protected_ids().is_empty()
            && (layer_id(layer).is_some_and(|id| self.protected_ids().contains_key(&id))
                || layer
                    .children()
                    .iter()
                    .any(|child| self.protects_layer(child)))
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn value(&self, set: usize, variable: &str) -> Option<&Value> {
        let id = self.definitions().get(variable)?.id;
        self.sets().get(set)?.values.get(&id)
    }

    fn require_editable(&self) -> Result<(), String> {
        if let Some(issue) = self.issues().first() {
            return Err(format!(
                "The original variable resources contain unsupported data: {issue}"
            ));
        }
        Ok(())
    }

    fn edit(&mut self) -> &mut Template {
        self.dirty = true;
        Arc::make_mut(&mut self.data)
    }

    pub fn rename_set(&mut self, index: usize, name: &str) -> Result<(), String> {
        self.require_editable()?;
        if index >= self.sets().len() {
            return Err("Unknown data set".into());
        }
        self.check_set_name(name, Some(index))?;
        if self.sets()[index].name != name {
            self.edit().sets[index].name = name.into();
        }
        Ok(())
    }

    pub fn delete_set(&mut self, index: usize) -> Result<(), String> {
        self.require_editable()?;
        if index >= self.sets().len() {
            return Err("Unknown data set".into());
        }
        self.edit().sets.remove(index);
        Ok(())
    }

    fn check_set_name(&self, name: &str, except: Option<usize>) -> Result<(), String> {
        if name.trim().is_empty() || !valid_xml_text(name) {
            return Err("Enter a valid data set name".into());
        }
        if self
            .sets()
            .iter()
            .enumerate()
            .any(|(idx, set)| Some(idx) != except && set.name == name)
        {
            return Err(format!("A data set named {name} already exists"));
        }
        Ok(())
    }

    pub fn import_sets(&mut self, sets: Vec<DataSet>, replace: bool) -> Result<bool, String> {
        self.require_editable()?;
        let mut names = BTreeSet::new();
        if !replace {
            names.extend(self.sets().iter().map(|set| set.name.as_str()));
        }
        for set in &sets {
            if set.name.trim().is_empty() || !valid_xml_text(&set.name) || !names.insert(&set.name)
            {
                return Err(format!("Invalid or duplicate data set name: {}", set.name));
            }
            if set.values.len() != self.definitions().len()
                || self.definitions().values().any(|variable| {
                    !set.values
                        .get(&variable.id)
                        .is_some_and(|value| valid_value(variable.kind, value))
                })
            {
                return Err(format!(
                    "Data set {} does not match the variables",
                    set.name
                ));
            }
        }
        if (replace && self.sets() == sets) || (!replace && sets.is_empty()) {
            return Ok(false);
        }
        let current = &mut self.edit().sets;
        if replace {
            *current = sets;
        } else {
            current.extend(sets);
        }
        Ok(true)
    }
}

fn count_protected_ids(
    stack: &LayerStack,
    protected: &BTreeMap<LayerId, usize>,
    counts: &mut BTreeMap<LayerId, usize>,
) {
    for layer in stack.iter() {
        if let Some(id) = layer_id(layer).filter(|id| protected.contains_key(id)) {
            *counts.entry(id).or_default() += 1;
        }
        count_protected_ids(layer.children(), protected, counts);
    }
}

fn valid_value(kind: VariableKind, value: &Value) -> bool {
    kind.accepts_value(value)
        && match value {
            Value::Text(text) => valid_xml_text(text),
            Value::File(file) => !file.trim().is_empty() && valid_xml_text(file),
            _ => true,
        }
}

fn at_path<'a>(mut stack: &'a LayerStack, path: &[usize]) -> Option<&'a Layer> {
    let (&last, parents) = path.split_last()?;
    for &parent in parents {
        stack = stack.get(parent)?.children();
    }
    stack.get(last)
}

fn first_eligible<'a>(stack: &'a LayerStack, variable: &Variable) -> Option<&'a Layer> {
    for layer in stack.as_slice().iter().rev() {
        if layer_id(layer).is_some_and(|id| {
            variable
                .bindings
                .iter()
                .any(|binding| binding.layer_id == id)
        }) && variable.kind.accepts(layer)
        {
            return Some(layer);
        }
        if let Some(layer) = first_eligible(layer.children(), variable) {
            return Some(layer);
        }
    }
    None
}

fn layer_value(kind: VariableKind, layer: &Layer) -> Value {
    match kind {
        VariableKind::Visibility => Value::Visibility(layer.visible),
        VariableKind::Text => {
            Value::Text(layer.type_tool().expect("eligible type layer").text.clone())
        }
        VariableKind::Pixels => Value::KeepPixels,
    }
}

fn prune(data: &mut Template, mut retain: impl FnMut(&Binding, VariableKind) -> bool) {
    data.definitions.retain(|_, variable| {
        variable
            .bindings
            .retain(|binding| retain(binding, variable.kind));
        if variable.bindings.is_empty() {
            for set in &mut data.sets {
                set.values.remove(&variable.id);
            }
            return false;
        }
        true
    });
    if data.definitions.is_empty() {
        data.sets.clear();
    }
}

impl Document {
    pub fn bind_variable(
        &mut self,
        path: &[usize],
        name: &str,
        kind: VariableKind,
        placement: Placement,
    ) -> Result<(), String> {
        self.variables.require_editable()?;
        if !valid_name(name) {
            return Err(format!("Invalid variable name: {name}"));
        }
        if !placement.valid() {
            return Err("Invalid pixel placement".into());
        }
        let layer = at_path(&self.layers, path)
            .filter(|layer| kind.accepts(layer))
            .ok_or("The layer cannot use this variable type")?;
        let initial = layer_value(kind, layer);
        if self
            .variables
            .definitions()
            .get(name)
            .is_some_and(|variable| variable.kind != kind)
        {
            return Err(format!("The variable {name} already has a different type"));
        }
        ensure_layer_ids(self, self.layer_comps.next_layer_id());
        let layer_id = at_path(&self.layers, path)
            .and_then(layer_id)
            .ok_or("A stable layer ID could not be assigned")?;
        let target = Binding {
            layer_id,
            placement,
        };
        if self
            .variables
            .definitions()
            .get(name)
            .is_some_and(|variable| variable.bindings.contains(&target))
        {
            return Ok(());
        }
        let data = self.variables.edit();
        let id = data
            .definitions
            .values()
            .map(|variable| variable.id)
            .max()
            .unwrap_or(0)
            + 1;
        let mut original = data.definitions.remove(name).unwrap_or(Variable {
            id,
            kind,
            bindings: Vec::new(),
        });
        // Keep the rows while moving a layer's last binding to another variable.
        for variable in data
            .definitions
            .values_mut()
            .filter(|variable| variable.kind == kind)
        {
            variable
                .bindings
                .retain(|binding| binding.layer_id != layer_id);
        }
        original
            .bindings
            .retain(|binding| binding.layer_id != layer_id);
        original.bindings.push(target);
        for set in &mut data.sets {
            set.values
                .entry(original.id)
                .or_insert_with(|| initial.clone());
        }
        data.definitions.insert(name.into(), original);
        prune(data, |_, _| true);
        Ok(())
    }

    pub fn unbind_variable(&mut self, path: &[usize], kind: VariableKind) -> Result<(), String> {
        self.variables.require_editable()?;
        let id = at_path(&self.layers, path)
            .ok_or("Unknown layer")
            .map(layer_id)?;
        if let Some(id) = id {
            if self.variables.definitions().values().any(|variable| {
                variable.kind == kind
                    && variable
                        .bindings
                        .iter()
                        .any(|binding| binding.layer_id == id)
            }) {
                prune(self.variables.edit(), |binding, bound_kind| {
                    binding.layer_id != id || kind != bound_kind
                });
            }
        }
        Ok(())
    }

    pub fn prune_variable_bindings(&mut self) {
        if self.variables.definitions().is_empty() || !self.variables.issues().is_empty() {
            return;
        }
        let remaining = layer_ids(self);
        if self
            .variables
            .definitions()
            .values()
            .flat_map(|v| &v.bindings)
            .any(|b| !remaining.contains(&b.layer_id))
        {
            prune(self.variables.edit(), |binding, _| {
                remaining.contains(&binding.layer_id)
            });
        }
    }

    pub fn rename_variable(&mut self, old: &str, name: &str) -> Result<(), String> {
        self.variables.require_editable()?;
        if !self.variables.definitions().contains_key(old) {
            return Err("Unknown variable".into());
        }
        if old == name {
            return Ok(());
        }
        if !valid_name(name) || self.variables.definitions().contains_key(name) {
            return Err(format!("Invalid or duplicate variable name: {name}"));
        }
        let definitions = &mut self.variables.edit().definitions;
        if let Some(variable) = definitions.remove(old) {
            definitions.insert(name.into(), variable);
        }
        Ok(())
    }

    pub fn new_data_set(&mut self, copy: Option<usize>, name: &str) -> Result<usize, String> {
        self.variables.require_editable()?;
        if self.variables.definitions().is_empty() {
            return Err("Define at least one variable first".into());
        }
        let name = if name.is_empty() {
            default_set_name(|name| self.variables.sets().iter().any(|set| set.name == name))
        } else {
            self.variables.check_set_name(name, None)?;
            name.into()
        };
        let values = if let Some(index) = copy {
            self.variables
                .sets()
                .get(index)
                .ok_or("Unknown data set")?
                .values
                .clone()
        } else {
            self.variables
                .definitions()
                .iter()
                .map(|(name, variable)| {
                    let layer = first_eligible(&self.layers, variable)
                        .ok_or_else(|| format!("Variable {name} has no eligible layer"))?;
                    Ok((variable.id, layer_value(variable.kind, layer)))
                })
                .collect::<Result<_, String>>()?
        };
        let index = self.variables.sets().len();
        self.variables.edit().sets.push(DataSet { name, values });
        Ok(index)
    }

    pub fn set_data_value(&mut self, set: usize, name: &str, value: Value) -> Result<(), String> {
        self.variables.require_editable()?;
        let variable = self
            .variables
            .definitions()
            .get(name)
            .ok_or("Unknown variable")?;
        if !valid_value(variable.kind, &value) {
            return Err(format!("Invalid value for variable {name}"));
        }
        if set >= self.variables.sets().len() {
            return Err("Unknown data set".into());
        }
        let id = variable.id;
        if self.variables.sets()[set].values.get(&id) != Some(&value) {
            self.variables.edit().sets[set].values.insert(id, value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
