//! The project's architecture as containers and the interactions its flows
//! imply: the second level of the C4 model, derived from what the map and the
//! overview already hold.
//!
//! A *container* is a top-level component of the map (what a person would draw
//! as a box: an app, a service, a store). Its technologies are the ones the
//! decisions that touch it also use. An *interaction* is a step of a flow
//! passing from one container to another; the step's title is its label, and
//! the flow and position say where it sits in a story (the C4 dynamic
//! diagram). Nothing here is invented: an interaction exists only because a
//! cited flow steps from one component of the map to another.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{EntityKind, NodeKind};
use serde::{Deserialize, Serialize};

use crate::graph::{ProjectGraph, ProjectMap};
use crate::overview::OverviewFlow;

/// Most containers drawn; the rest are counted in [`Architecture::hidden`].
pub const MAX_CONTAINERS: usize = 12;
/// Most technologies listed on one container.
const MAX_TECHNOLOGIES: usize = 3;
/// Longest role line, in characters.
const ROLE_CHARS: usize = 90;
/// Climbs of `part_of` before giving up (a cycle in the data).
const MAX_DEPTH: usize = 16;

/// A top-level component and what it is drawn with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    /// The component's entity id.
    pub entity_id: String,
    /// Its name.
    pub name: String,
    /// One line on what it is, from its description (may be empty).
    pub role: String,
    /// Technologies its decisions use, most used first.
    pub technologies: Vec<String>,
    /// Components inside it.
    pub parts: usize,
    /// Decisions in force tied to it.
    pub decisions: usize,
    /// Conflicting decisions on it.
    pub conflicts: usize,
    /// Positions (in the overview's flows) of the flows that pass through it.
    pub flows: Vec<usize>,
}

/// Where in a flow an interaction happens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRef {
    /// Position of the flow.
    pub flow: usize,
    /// Position of the step that arrives at the target.
    pub step: usize,
    /// The step's title: what travels or happens.
    pub title: String,
    /// The protocol or medium it travels by, when the flow names one.
    #[serde(default)]
    pub via: Option<String>,
}

/// A step of a flow passing between two containers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interaction {
    /// Container it leaves (entity id).
    pub from: String,
    /// Container it reaches (entity id).
    pub to: String,
    /// The flow steps that make it, in the order found.
    pub steps: Vec<StepRef>,
}

/// A container on the other end of an interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbour {
    /// Position of the container in [`Architecture::containers`].
    pub index: usize,
    /// The flow steps that make the interaction.
    pub steps: Vec<StepRef>,
}

impl Neighbour {
    /// The first medium a step names, when any does.
    pub fn via(&self) -> Option<&str> {
        self.steps.iter().find_map(|step| step.via.as_deref())
    }
}

/// The containers of a project and how its flows move between them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Architecture {
    /// Containers shown, those the flows pass through first.
    pub containers: Vec<Container>,
    /// Interactions between shown containers.
    pub interactions: Vec<Interaction>,
    /// Top-level components left out of `containers` (over the limit).
    #[serde(default)]
    pub hidden: usize,
}

impl Architecture {
    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }

    /// The container with this entity id.
    pub fn container(&self, entity_id: &str) -> Option<&Container> {
        self.containers
            .iter()
            .find(|container| container.entity_id == entity_id)
    }

    /// Position of the container with this entity id.
    pub fn position(&self, entity_id: &str) -> Option<usize> {
        self.containers
            .iter()
            .position(|container| container.entity_id == entity_id)
    }

    /// The interaction that goes from the container at `from` to the one at
    /// `to`, when some flow steps that way.
    pub fn interaction(&self, from: usize, to: usize) -> Option<&Interaction> {
        let (from, to) = (self.containers.get(from)?, self.containers.get(to)?);
        self.interactions.iter().find(|interaction| {
            interaction.from == from.entity_id && interaction.to == to.entity_id
        })
    }

    /// Only what one flow touches: its containers (in the same order) and
    /// the steps of that flow, so a drawing of the flow carries no part it
    /// does not pass through.
    pub fn narrowed_to(&self, flow: usize) -> Architecture {
        Architecture {
            containers: self
                .containers
                .iter()
                .filter(|container| container.flows.contains(&flow))
                .cloned()
                .collect(),
            interactions: self
                .interactions
                .iter()
                .filter_map(|interaction| {
                    let steps: Vec<StepRef> = interaction
                        .steps
                        .iter()
                        .filter(|step| step.flow == flow)
                        .cloned()
                        .collect();
                    (!steps.is_empty()).then(|| Interaction {
                        from: interaction.from.clone(),
                        to: interaction.to.clone(),
                        steps,
                    })
                })
                .collect(),
            hidden: 0,
        }
    }

    /// What the container at `index` calls, the most used first.
    pub fn calls(&self, index: usize) -> Vec<Neighbour> {
        self.neighbours(index, true)
    }

    /// What calls the container at `index`, the most used first.
    pub fn called_by(&self, index: usize) -> Vec<Neighbour> {
        self.neighbours(index, false)
    }

    fn neighbours(&self, index: usize, outgoing: bool) -> Vec<Neighbour> {
        let Some(own) = self.containers.get(index) else {
            return Vec::new();
        };
        let mut found: Vec<Neighbour> = self
            .interactions
            .iter()
            .filter_map(|interaction| {
                let (mine, other) = if outgoing {
                    (&interaction.from, &interaction.to)
                } else {
                    (&interaction.to, &interaction.from)
                };
                (*mine == own.entity_id).then_some(())?;
                Some(Neighbour {
                    index: self.position(other)?,
                    steps: interaction.steps.clone(),
                })
            })
            .collect();
        found.sort_by_key(|neighbour| (std::cmp::Reverse(neighbour.steps.len()), neighbour.index));
        found
    }

    /// The interactions of one flow, in step order.
    pub fn of_flow(&self, flow: usize) -> Vec<(&Interaction, &StepRef)> {
        let mut found: Vec<(&Interaction, &StepRef)> = self
            .interactions
            .iter()
            .flat_map(|interaction| {
                interaction
                    .steps
                    .iter()
                    .filter(move |step| step.flow == flow)
                    .map(move |step| (interaction, step))
            })
            .collect();
        found.sort_by_key(|(_, step)| step.step);
        found
    }
}

fn clip(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// Derives the architecture from the map, the graph's confirmed edges and the
/// overview's flows.
pub fn derive(map: &ProjectMap, graph: &ProjectGraph, flows: &[OverviewFlow]) -> Architecture {
    let components: BTreeMap<&str, &crate::graph::MapEntity> = map
        .entities
        .iter()
        .filter(|row| row.entity.kind == EntityKind::Component)
        .map(|row| (row.entity.entity_id.as_str(), row))
        .collect();
    let parent: BTreeMap<&str, &str> = map
        .part_of
        .iter()
        .map(|(child, parent)| (child.as_str(), parent.as_str()))
        .collect();
    let top = |id: &str| -> Option<&str> {
        let (&start, _) = components.get_key_value(id)?;
        let mut at = start;
        for _ in 0..MAX_DEPTH {
            match parent.get(at) {
                Some(next) if components.contains_key(next) => at = next,
                _ => break,
            }
        }
        Some(at)
    };

    // What the flows say: the containers they pass through and the steps that
    // go from one to another.
    let mut in_flows: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    let mut pairs: Vec<((String, String), Vec<StepRef>)> = Vec::new();
    for (flow_at, flow) in flows.iter().enumerate() {
        let mut previous: Option<&str> = None;
        for (step_at, step) in flow.steps.iter().enumerate() {
            let Some(container) = step.entity_id.as_deref().and_then(top) else {
                continue;
            };
            in_flows.entry(container).or_default().insert(flow_at);
            if let Some(from) = previous.filter(|from| *from != container) {
                let step_ref = StepRef {
                    flow: flow_at,
                    step: step_at,
                    title: step.title.clone(),
                    via: step.via.clone(),
                };
                let key = (from.to_owned(), container.to_owned());
                match pairs.iter_mut().find(|(known, _)| *known == key) {
                    Some((_, steps)) => steps.push(step_ref),
                    None => pairs.push((key, vec![step_ref])),
                }
            }
            previous = Some(container);
        }
    }

    // Technologies: a decision that affects a component and uses a technology
    // ties the two.
    let technology_names: BTreeMap<&str, &str> = map
        .entities
        .iter()
        .filter(|row| row.entity.kind == EntityKind::Technology)
        .map(|row| (row.entity.entity_id.as_str(), row.entity.name.as_str()))
        .collect();
    let mut affects: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut uses: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &graph.edges {
        if edge.from.kind != NodeKind::Decision || edge.to.kind != NodeKind::Entity {
            continue;
        }
        match edge.kind.as_str() {
            "affects" => affects
                .entry(edge.from.id.as_str())
                .or_default()
                .push(edge.to.id.as_str()),
            "uses" => uses
                .entry(edge.from.id.as_str())
                .or_default()
                .push(edge.to.id.as_str()),
            _ => {}
        }
    }
    let mut technology_use: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for (decision, touched) in &affects {
        let Some(used) = uses.get(decision) else {
            continue;
        };
        for container in touched.iter().filter_map(|id| top(id)) {
            for technology in used {
                if let Some(name) = technology_names.get(technology) {
                    *technology_use
                        .entry(container)
                        .or_default()
                        .entry(name)
                        .or_default() += 1;
                }
            }
        }
    }

    // Parts per container.
    let mut parts: BTreeMap<&str, usize> = BTreeMap::new();
    for id in components.keys() {
        if let Some(container) = top(id).filter(|container| container != id) {
            *parts.entry(container).or_default() += 1;
        }
    }

    // The containers: top-level components, those in flows first, then by
    // how much was decided about them.
    let mut tops: Vec<&crate::graph::MapEntity> = components
        .iter()
        .filter(|(id, _)| top(id) == Some(**id))
        .map(|(_, row)| *row)
        .collect();
    tops.sort_by(|a, b| {
        let flowed =
            |row: &crate::graph::MapEntity| in_flows.contains_key(row.entity.entity_id.as_str());
        (!flowed(a), std::cmp::Reverse(a.decisions), &a.entity.name).cmp(&(
            !flowed(b),
            std::cmp::Reverse(b.decisions),
            &b.entity.name,
        ))
    });
    let hidden = tops.len().saturating_sub(MAX_CONTAINERS);
    tops.truncate(MAX_CONTAINERS);

    let containers: Vec<Container> = tops
        .iter()
        .map(|row| {
            let id = row.entity.entity_id.as_str();
            let mut technologies: Vec<(&str, usize)> = technology_use
                .get(id)
                .map(|used| used.iter().map(|(name, count)| (*name, *count)).collect())
                .unwrap_or_default();
            technologies.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            Container {
                entity_id: id.to_owned(),
                name: row.entity.name.clone(),
                role: clip(&row.entity.description, ROLE_CHARS),
                technologies: technologies
                    .into_iter()
                    .take(MAX_TECHNOLOGIES)
                    .map(|(name, _)| name.to_owned())
                    .collect(),
                parts: parts.get(id).copied().unwrap_or(0),
                decisions: row.decisions,
                conflicts: row.conflicts,
                flows: in_flows
                    .get(id)
                    .map(|set| set.iter().copied().collect())
                    .unwrap_or_default(),
            }
        })
        .collect();
    let shown: BTreeSet<&str> = containers
        .iter()
        .map(|container| container.entity_id.as_str())
        .collect();
    let interactions = pairs
        .into_iter()
        .filter(|((from, to), _)| shown.contains(from.as_str()) && shown.contains(to.as_str()))
        .map(|((from, to), steps)| Interaction { from, to, steps })
        .collect();
    Architecture {
        containers,
        interactions,
        hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EntityRecord, GraphEdge, MapEntity, NodeRef};
    use crate::overview::OverviewStep;

    fn entity(id: &str, kind: EntityKind, decisions: usize) -> MapEntity {
        MapEntity {
            entity: EntityRecord {
                entity_id: id.into(),
                project_id: "p".into(),
                kind,
                name: id.into(),
                key: id.into(),
                description: format!("{id} faz o seu trabalho"),
                patterns: vec![],
                aliases: vec![],
                created_at: "2026-01-01T00:00:00Z".into(),
                retired_at: None,
            },
            decisions,
            claims: 0,
            last_activity: None,
            conflicts: 0,
        }
    }

    fn step(title: &str, component: Option<&str>) -> OverviewStep {
        OverviewStep {
            title: title.into(),
            text: String::new(),
            entity_id: component.map(str::to_owned),
            entity_name: component.map(str::to_owned),
            via: None,
            citations: vec![],
        }
    }

    fn edge(from: NodeRef, to: NodeRef, kind: &str) -> GraphEdge {
        GraphEdge {
            from,
            to,
            kind: kind.into(),
        }
    }

    fn fixture() -> (ProjectMap, ProjectGraph, Vec<OverviewFlow>) {
        let map = ProjectMap {
            as_of: "2026-10-02T00:00:00Z".into(),
            entities: vec![
                entity("api", EntityKind::Component, 1),
                entity("jobs", EntityKind::Component, 2),
                entity("jobs-queue", EntityKind::Component, 0),
                entity("store", EntityKind::Component, 3),
                entity("docs", EntityKind::Component, 9),
                entity("sqlite", EntityKind::Technology, 3),
            ],
            part_of: vec![("jobs-queue".into(), "jobs".into())],
            pending: 0,
        };
        let graph = ProjectGraph {
            nodes: vec![],
            edges: vec![
                edge(NodeRef::decision("d1"), NodeRef::entity("store"), "affects"),
                edge(NodeRef::decision("d1"), NodeRef::entity("sqlite"), "uses"),
                edge(NodeRef::decision("d2"), NodeRef::entity("store"), "affects"),
                edge(NodeRef::decision("d2"), NodeRef::entity("sqlite"), "uses"),
            ],
            suggested: vec![],
        };
        let flows = vec![OverviewFlow {
            title: "Captura".into(),
            description: String::new(),
            steps: vec![
                step("Chega a captura", Some("api")),
                step("Entra na fila", Some("jobs-queue")),
                step("Sem componente", None),
                step("Grava", Some("store")),
                step("Confirma", Some("store")),
            ],
        }];
        (map, graph, flows)
    }

    #[test]
    fn flows_become_interactions_between_top_level_containers() {
        let (map, graph, flows) = fixture();
        let architecture = derive(&map, &graph, &flows);
        let names: Vec<&str> = architecture
            .containers
            .iter()
            .map(|container| container.entity_id.as_str())
            .collect();
        // Containers in the flow first (by what was decided), then the rest.
        assert_eq!(names, vec!["store", "jobs", "api", "docs"]);
        let pairs: Vec<(&str, &str)> = architecture
            .interactions
            .iter()
            .map(|interaction| (interaction.from.as_str(), interaction.to.as_str()))
            .collect();
        // A part counts as its container; a step without a component is
        // skipped; two steps in the same container make no interaction.
        assert_eq!(pairs, vec![("api", "jobs"), ("jobs", "store")]);
        let reach = architecture.of_flow(0);
        assert_eq!(reach.len(), 2);
        assert_eq!(reach[0].1.title, "Entra na fila");
        assert_eq!(reach[1].1.title, "Grava");
        assert_eq!(architecture.container("jobs").unwrap().parts, 1);
    }

    #[test]
    fn technologies_come_from_decisions_that_touch_and_use() {
        let (map, graph, flows) = fixture();
        let architecture = derive(&map, &graph, &flows);
        assert_eq!(
            architecture.container("store").unwrap().technologies,
            vec!["sqlite"]
        );
        assert!(architecture
            .container("api")
            .unwrap()
            .technologies
            .is_empty());
    }

    fn container_named(id: &str) -> Container {
        Container {
            entity_id: id.into(),
            name: id.into(),
            ..Container::default()
        }
    }

    fn interaction(from: &str, to: &str, steps: &[(usize, Option<&str>)]) -> Interaction {
        Interaction {
            from: from.into(),
            to: to.into(),
            steps: steps
                .iter()
                .map(|(step, via)| StepRef {
                    flow: 0,
                    step: *step,
                    title: format!("passo {step}"),
                    via: via.map(str::to_owned),
                })
                .collect(),
        }
    }

    #[test]
    fn neighbours_list_who_calls_and_who_is_called_most_used_first() {
        let architecture = Architecture {
            containers: vec![
                container_named("api"),
                container_named("jobs"),
                container_named("store"),
            ],
            interactions: vec![
                interaction("api", "jobs", &[(1, Some("fila"))]),
                interaction("api", "store", &[(2, None), (3, Some("SQL"))]),
                interaction("jobs", "store", &[(4, None)]),
            ],
            hidden: 0,
        };
        let calls = architecture.calls(0);
        assert_eq!(
            calls.iter().map(|n| n.index).collect::<Vec<_>>(),
            vec![2, 1],
            "two steps beat one"
        );
        assert_eq!(
            calls[0].via(),
            Some("SQL"),
            "the first medium any step names"
        );
        assert_eq!(calls[1].via(), Some("fila"));
        let into_store = architecture.called_by(2);
        assert_eq!(
            into_store.iter().map(|n| n.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(architecture.called_by(0).is_empty());
        assert!(architecture.calls(2).is_empty());
        assert!(architecture.calls(9).is_empty(), "no such container");
        assert_eq!(
            architecture.interaction(0, 2).map(|i| i.steps.len()),
            Some(2)
        );
        assert!(architecture.interaction(2, 0).is_none());
    }

    #[test]
    fn a_flow_keeps_only_its_own_parts_and_steps() {
        let mut architecture = Architecture {
            containers: vec![
                container_named("api"),
                container_named("jobs"),
                container_named("store"),
                container_named("docs"),
            ],
            interactions: vec![
                interaction("api", "jobs", &[(1, None)]),
                interaction("jobs", "store", &[(2, None)]),
            ],
            hidden: 2,
        };
        architecture.containers[0].flows = vec![0, 1];
        architecture.containers[1].flows = vec![0];
        architecture.containers[2].flows = vec![1];
        architecture.interactions[1].steps[0].flow = 1;
        let first = architecture.narrowed_to(0);
        assert_eq!(
            first
                .containers
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec!["api", "jobs"],
            "docs and store are not in the flow"
        );
        assert_eq!(first.interactions.len(), 1);
        assert_eq!(first.hidden, 0);
        let second = architecture.narrowed_to(1);
        assert_eq!(second.containers.len(), 2);
        assert!(second.interaction(0, 1).is_none(), "api to jobs is flow 0");
    }

    #[test]
    fn too_many_containers_are_counted_not_drawn() {
        let (mut map, graph, flows) = fixture();
        for n in 0..20 {
            map.entities
                .push(entity(&format!("extra{n}"), EntityKind::Component, 0));
        }
        let architecture = derive(&map, &graph, &flows);
        assert_eq!(architecture.containers.len(), MAX_CONTAINERS);
        assert_eq!(architecture.hidden, 24 - MAX_CONTAINERS);
    }

    #[test]
    fn an_empty_map_is_an_empty_architecture() {
        let map = ProjectMap {
            as_of: String::new(),
            entities: vec![],
            part_of: vec![],
            pending: 0,
        };
        let graph = ProjectGraph {
            nodes: vec![],
            edges: vec![],
            suggested: vec![],
        };
        assert!(derive(&map, &graph, &[]).is_empty());
    }
}
