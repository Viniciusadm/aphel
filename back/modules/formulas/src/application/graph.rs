use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::application::{CompiledFormula, FormulaContext, FormulaWorkspace, compile_formula};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::FieldDefinitionReader;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::fields::FieldDefinition;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormulaNode {
    pub record_type: RecordType,
    pub key: String,
}

impl FormulaNode {
    pub fn label(&self) -> String {
        format!("{}.{}", self.record_type.as_str(), self.key)
    }
}

pub fn formula_edges(
    context: &FormulaContext,
    compiled: &CompiledFormula,
) -> BTreeSet<FormulaNode> {
    let mut edges = BTreeSet::new();
    for key in &compiled.dependencies.fields {
        if context.field(key).is_some_and(|field| field.is_formula()) {
            edges.insert(FormulaNode {
                record_type: context.record_type.record_type.clone(),
                key: key.clone(),
            });
        }
    }
    for aggregate in &compiled.dependencies.aggregates {
        let Some(relation) = context.relation(&aggregate.relation) else {
            continue;
        };
        let (Some(target), Some(field)) = (relation.source.target_type(), &aggregate.field) else {
            continue;
        };
        if relation
            .field(field)
            .is_some_and(|related| related.is_formula)
        {
            edges.insert(FormulaNode {
                record_type: target.clone(),
                key: field.clone(),
            });
        }
    }
    edges
}

pub fn order_formulas(context: &FormulaContext) -> Vec<(FieldDefinition, Option<CompiledFormula>)> {
    let mut compiled: BTreeMap<String, (FieldDefinition, Option<CompiledFormula>)> = context
        .formulas()
        .map(|field| {
            let definition = field.definition().clone();
            let formula = definition
                .formula()
                .and_then(|settings| compile_formula(&settings.expression, context).ok());
            (field.key().to_owned(), (definition, formula))
        })
        .collect();
    let mut pending: BTreeMap<String, BTreeSet<String>> = compiled
        .iter()
        .map(|(key, (_, formula))| {
            let local = formula
                .as_ref()
                .map(|formula| {
                    formula
                        .dependencies
                        .fields
                        .iter()
                        .filter(|dependency| {
                            *dependency != key && compiled.contains_key(*dependency)
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            (key.clone(), local)
        })
        .collect();
    let mut ordered = Vec::with_capacity(compiled.len());
    loop {
        let ready: Vec<String> = pending
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(key, _)| key.clone())
            .collect();
        if ready.is_empty() {
            break;
        }
        for key in ready {
            pending.remove(&key);
            for dependencies in pending.values_mut() {
                dependencies.remove(&key);
            }
            if let Some(entry) = compiled.remove(&key) {
                ordered.push(entry);
            }
        }
    }
    ordered.extend(
        compiled
            .into_values()
            .map(|(definition, _)| (definition, None)),
    );
    ordered
}

pub async fn find_cycle<D>(
    workspace: &mut FormulaWorkspace<'_, D>,
    start: FormulaNode,
) -> Result<Option<Vec<FormulaNode>>, AppError>
where
    D: FieldDefinitionReader + ?Sized,
{
    let mut contexts: HashMap<RecordType, FormulaContext> = HashMap::new();
    let mut explored: BTreeSet<FormulaNode> = BTreeSet::new();
    let first = node_edges(workspace, &mut contexts, &start).await?;
    let mut stack: Vec<(FormulaNode, Vec<FormulaNode>, usize)> = vec![(start.clone(), first, 0)];
    while let Some((_, neighbors, index)) = stack.last_mut() {
        let Some(next) = neighbors.get(*index).cloned() else {
            if let Some((node, _, _)) = stack.pop() {
                explored.insert(node);
            }
            continue;
        };
        *index += 1;
        if next == start {
            let mut path: Vec<FormulaNode> =
                stack.iter().map(|(node, _, _)| node.clone()).collect();
            path.push(start);
            return Ok(Some(path));
        }
        if explored.contains(&next) || stack.iter().any(|(node, _, _)| *node == next) {
            continue;
        }
        let edges = node_edges(workspace, &mut contexts, &next).await?;
        stack.push((next, edges, 0));
    }
    Ok(None)
}

async fn node_edges<D>(
    workspace: &mut FormulaWorkspace<'_, D>,
    contexts: &mut HashMap<RecordType, FormulaContext>,
    node: &FormulaNode,
) -> Result<Vec<FormulaNode>, AppError>
where
    D: FieldDefinitionReader + ?Sized,
{
    if !contexts.contains_key(&node.record_type) {
        if workspace.registry().get(&node.record_type).is_none() {
            return Ok(Vec::new());
        }
        let context = workspace.context(&node.record_type).await?;
        contexts.insert(node.record_type.clone(), context);
    }
    let Some(context) = contexts.get(&node.record_type) else {
        return Ok(Vec::new());
    };
    let Some(settings) = context
        .field(&node.key)
        .and_then(|field| field.definition().formula().cloned())
    else {
        return Ok(Vec::new());
    };
    Ok(compile_formula(&settings.expression, context)
        .map(|compiled| formula_edges(context, &compiled).into_iter().collect())
        .unwrap_or_default())
}
