//! What the page says about *why* the system is built as it is: the decisions
//! in force and the rules valid now, each tied to the parts of the map it
//! touches and to the decisions it relates to.
//!
//! Everything here is recorded and confirmed by a person: decisions, their
//! structured context, the confirmed edges of the map (suggestions waiting
//! for review are left out) and the human relations between decisions. The
//! only derived fields are mechanical: the short label, the container each
//! touched component sits in, and the first sentence of the rationale.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{EntityKind, NodeKind};
use domain::relations::RelationKind;
use serde::Serialize;

use crate::architecture::Architecture;
use crate::claims::ClaimRecord;
use crate::decisions::{DecisionStatus, StoredDecision};
use crate::extract::SIGNIFICANCE_CRITERIA;
use crate::graph::{ProjectGraph, ProjectMap};
use crate::injection::short_ref;
use crate::relations::RelationRow;

/// Most decisions in force on the page, newest first.
pub const MAX_PAGE_DECISIONS: usize = 200;
/// Most rules on the page.
pub const MAX_PAGE_RULES: usize = 80;
/// Longest one-line reason, in characters.
const REASON_CHARS: usize = 160;
/// Climbs of `part_of` before giving up (a cycle in the data).
const MAX_DEPTH: usize = 16;

/// The decisions and rules the page shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageKnowledge {
    /// Decisions in force, newest first.
    pub decisions: Vec<PageDecision>,
    /// Rules valid now.
    pub rules: Vec<PageRule>,
}

/// A decision in force, with its context and where it lands on the map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageDecision {
    /// Decision id.
    pub id: String,
    /// Short label, as the overview cites it (`D:xxxxxxxx`).
    pub label: String,
    /// The question it answers.
    pub question: String,
    /// The choice made.
    pub choice: String,
    /// Why, in full.
    pub rationale: String,
    /// The first sentence of the rationale, for one-line reading.
    pub reason: String,
    /// What it takes as true.
    pub assumptions: Vec<String>,
    /// What follows from it.
    pub consequences: Vec<String>,
    /// When to look at it again.
    pub reconsider_when: Vec<String>,
    /// Where it applies.
    pub scope: Vec<String>,
    /// Significance criteria the extraction ticked, in canonical order; empty
    /// for decisions recorded before criteria existed.
    pub criteria: Vec<String>,
    /// Entities of the map it affects or uses (confirmed edges only).
    pub entities: Vec<PageEntity>,
    /// Containers of the architecture it touches, in the architecture's order.
    pub containers: Vec<String>,
    /// Relations to other decisions.
    pub relations: Vec<PageRelation>,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
}

/// A rule valid now and where it applies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageRule {
    /// Claim id.
    pub id: String,
    /// Short label (`R:xxxxxxxx`).
    pub label: String,
    /// `assumption`, `constraint`, `goal` or `convention`.
    pub kind: String,
    /// The statement.
    pub statement: String,
    /// Entities it applies to (confirmed edges only).
    pub entities: Vec<PageEntity>,
    /// Containers of the architecture it touches.
    pub containers: Vec<String>,
}

/// An entity a record is tied to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageEntity {
    /// Entity id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// `component` or `technology`.
    pub kind: String,
    /// The edge: `affects`, `uses` or `applies_to`.
    pub edge: String,
}

/// A relation seen from one decision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageRelation {
    /// `supersedes`, `depends_on` or `conflicts_with`.
    pub kind: String,
    /// Whether this decision is the source (always true for a conflict,
    /// which reads the same both ways).
    pub outgoing: bool,
    /// The other decision.
    pub other_id: String,
    /// Its short label.
    pub other_label: String,
    /// Its question, when known.
    pub other_title: String,
    /// Whether the other decision is on the page (in force).
    pub in_force: bool,
}

/// Assembles the page's decisions and rules.
///
/// `decisions` may hold superseded ones: they only name the other end of a
/// relation. `claims` must already be the rules valid now; `criteria` maps a
/// decision id to the criteria of the candidate it came from.
pub fn assemble(
    decisions: &[StoredDecision],
    criteria: &BTreeMap<String, Vec<String>>,
    claims: &[ClaimRecord],
    map: &ProjectMap,
    graph: &ProjectGraph,
    relations: &[RelationRow],
    architecture: &Architecture,
) -> PageKnowledge {
    let superseded: BTreeSet<&str> = relations
        .iter()
        .filter(|row| row.kind == RelationKind::Supersedes.as_str())
        .map(|row| row.to.as_str())
        .collect();
    let mut in_force: Vec<&StoredDecision> = decisions
        .iter()
        .filter(|decision| {
            decision.status == DecisionStatus::Accepted
                && !superseded.contains(decision.decision_id.as_str())
        })
        .collect();
    in_force.sort_by(|left, right| {
        (&right.confirmed_at, &right.decision_id).cmp(&(&left.confirmed_at, &left.decision_id))
    });
    in_force.truncate(MAX_PAGE_DECISIONS);
    let shown: BTreeSet<&str> = in_force
        .iter()
        .map(|decision| decision.decision_id.as_str())
        .collect();
    let questions: BTreeMap<&str, &str> = decisions
        .iter()
        .map(|decision| (decision.decision_id.as_str(), decision.question.as_str()))
        .collect();

    let places = Places::new(map, architecture);
    let mut tied: BTreeMap<(NodeKind, String), Vec<PageEntity>> = BTreeMap::new();
    for edge in &graph.edges {
        if edge.to.kind != NodeKind::Entity
            || !matches!(edge.kind.as_str(), "affects" | "uses" | "applies_to")
        {
            continue;
        }
        let Some(entity) = places.entity(&edge.to.id, &edge.kind) else {
            continue;
        };
        let list = tied
            .entry((edge.from.kind, edge.from.id.clone()))
            .or_default();
        if !list.iter().any(|known| known.id == entity.id) {
            list.push(entity);
        }
    }
    let mut take = |kind: NodeKind, id: &str| -> (Vec<PageEntity>, Vec<String>) {
        let entities = tied.remove(&(kind, id.to_owned())).unwrap_or_default();
        let containers = places.containers(&entities);
        (entities, containers)
    };

    let decisions = in_force
        .iter()
        .map(|decision| {
            let id = decision.decision_id.as_str();
            let (entities, containers) = take(NodeKind::Decision, id);
            PageDecision {
                id: id.to_owned(),
                label: format!("D:{}", short_ref(id)),
                question: decision.question.clone(),
                choice: decision.choice.clone(),
                rationale: decision.rationale.clone(),
                reason: first_sentence(&decision.rationale),
                assumptions: list(&decision.assumptions),
                consequences: list(&decision.consequences),
                reconsider_when: list(&decision.reconsider_when),
                scope: list(&decision.scope),
                criteria: canonical(criteria.get(id)),
                entities,
                containers,
                relations: relations_of(id, relations, &shown, &questions),
                confirmed_at: decision.confirmed_at.clone(),
            }
        })
        .collect();
    let rules = claims
        .iter()
        .take(MAX_PAGE_RULES)
        .map(|claim| {
            let (entities, containers) = take(NodeKind::Claim, &claim.claim_id);
            PageRule {
                id: claim.claim_id.clone(),
                label: format!("R:{}", short_ref(&claim.claim_id)),
                kind: claim.kind.as_str().to_owned(),
                statement: claim.statement.clone(),
                entities,
                containers,
            }
        })
        .collect();
    PageKnowledge { decisions, rules }
}

/// Where entities sit: their names and kinds, and the container of each
/// component (itself or the first ancestor the architecture draws).
struct Places<'a> {
    entities: BTreeMap<&'a str, (&'a str, EntityKind)>,
    parent: BTreeMap<&'a str, &'a str>,
    order: BTreeMap<&'a str, usize>,
}

impl<'a> Places<'a> {
    fn new(map: &'a ProjectMap, architecture: &'a Architecture) -> Self {
        Self {
            entities: map
                .entities
                .iter()
                .map(|row| {
                    let entity = &row.entity;
                    (
                        entity.entity_id.as_str(),
                        (entity.name.as_str(), entity.kind),
                    )
                })
                .collect(),
            parent: map
                .part_of
                .iter()
                .map(|(child, parent)| (child.as_str(), parent.as_str()))
                .collect(),
            order: architecture
                .containers
                .iter()
                .enumerate()
                .map(|(at, container)| (container.entity_id.as_str(), at))
                .collect(),
        }
    }

    fn entity(&self, id: &str, edge: &str) -> Option<PageEntity> {
        let (name, kind) = self.entities.get(id)?;
        Some(PageEntity {
            id: id.to_owned(),
            name: (*name).to_owned(),
            kind: match kind {
                EntityKind::Component => "component",
                EntityKind::Technology => "technology",
            }
            .to_owned(),
            edge: edge.to_owned(),
        })
    }

    fn container(&self, id: &str) -> Option<&'a str> {
        let mut at = *self.entities.get_key_value(id)?.0;
        for _ in 0..=MAX_DEPTH {
            if let Some((&found, _)) = self.order.get_key_value(at) {
                return Some(found);
            }
            at = *self.parent.get(at)?;
        }
        None
    }

    fn containers(&self, entities: &[PageEntity]) -> Vec<String> {
        let mut found: Vec<&str> = entities
            .iter()
            .filter(|entity| entity.kind == "component")
            .filter_map(|entity| self.container(&entity.id))
            .collect();
        found.sort_by_key(|id| self.order.get(id).copied().unwrap_or(usize::MAX));
        found.dedup();
        found.into_iter().map(str::to_owned).collect()
    }
}

fn relations_of(
    id: &str,
    relations: &[RelationRow],
    shown: &BTreeSet<&str>,
    questions: &BTreeMap<&str, &str>,
) -> Vec<PageRelation> {
    let mut found: Vec<PageRelation> = Vec::new();
    for row in relations {
        let Some(kind) = RelationKind::parse(&row.kind) else {
            continue;
        };
        let outgoing = row.from == id;
        if !outgoing && row.to != id {
            continue;
        }
        let other = if outgoing { &row.to } else { &row.from };
        if found
            .iter()
            .any(|known| known.kind == kind.as_str() && known.other_id == *other)
        {
            continue;
        }
        found.push(PageRelation {
            kind: kind.as_str().to_owned(),
            outgoing: outgoing || kind.is_symmetric(),
            other_id: other.clone(),
            other_label: format!("D:{}", short_ref(other)),
            other_title: questions
                .get(other.as_str())
                .map(|question| (*question).to_owned())
                .unwrap_or_default(),
            in_force: shown.contains(other.as_str()),
        });
    }
    found
}

/// A stored JSON array of strings; malformed or blank entries are skipped.
fn list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Known criteria only, in the order of [`SIGNIFICANCE_CRITERIA`].
fn canonical(criteria: Option<&Vec<String>>) -> Vec<String> {
    let Some(criteria) = criteria else {
        return Vec::new();
    };
    SIGNIFICANCE_CRITERIA
        .iter()
        .filter(|known| criteria.iter().any(|ticked| ticked == *known))
        .map(|known| (*known).to_owned())
        .collect()
}

/// The first sentence of `text`, on one line, without its final stop, and
/// clipped to [`REASON_CHARS`]; its first letter is lowered unless the
/// first word is all capitals (an acronym or a name such as `SQLite` stays).
pub fn first_sentence(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut end = flat.len();
    let chars: Vec<(usize, char)> = flat.char_indices().collect();
    for (at, (index, character)) in chars.iter().enumerate() {
        if matches!(character, '.' | '!' | '?' | ';')
            && chars.get(at + 1).is_none_or(|(_, next)| *next == ' ')
        {
            end = *index;
            break;
        }
    }
    let sentence = flat[..end].trim();
    let mut out: String = if sentence.chars().count() > REASON_CHARS {
        let cut: String = sentence.chars().take(REASON_CHARS - 1).collect();
        format!("{}…", cut.trim_end())
    } else {
        sentence.to_owned()
    };
    let first_word = out.split(' ').next().unwrap_or("");
    let keep = first_word.chars().skip(1).any(char::is_uppercase);
    if !keep {
        if let Some(first) = out.chars().next() {
            let lowered: String = first.to_lowercase().collect();
            out.replace_range(..first.len_utf8(), &lowered);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::Container;
    use crate::graph::{EntityRecord, GraphEdge, MapEntity, NodeRef};
    use domain::claims::ClaimKind;

    fn entity(id: &str, kind: EntityKind) -> MapEntity {
        MapEntity {
            entity: EntityRecord {
                entity_id: id.into(),
                project_id: "p".into(),
                kind,
                name: format!("{id} name"),
                key: id.into(),
                description: String::new(),
                patterns: vec![],
                aliases: vec![],
                created_at: "2026-01-01T00:00:00Z".into(),
                retired_at: None,
            },
            decisions: 0,
            claims: 0,
            last_activity: None,
            conflicts: 0,
        }
    }

    fn decision(id: &str, status: DecisionStatus, at: &str) -> StoredDecision {
        StoredDecision {
            decision_id: id.into(),
            candidate_id: format!("c-{id}"),
            project_id: "p".into(),
            project_location: "/p".into(),
            capture_id: None,
            status,
            question: format!("Pergunta {id}?"),
            choice: format!("Escolha {id}"),
            rationale: "O app pode estar fechado. Por isso o hook grava antes.".into(),
            assumptions: r#"["O disco local é confiável", " "]"#.into(),
            reconsider_when: r#"["Se o app virar serviço"]"#.into(),
            scope: "[]".into(),
            consequences: "not json".into(),
            version: 1,
            created_at: at.into(),
            confirmed_at: at.into(),
            updated_at: at.into(),
        }
    }

    fn edge(from: NodeRef, to: &str, kind: &str) -> GraphEdge {
        GraphEdge {
            from,
            to: NodeRef::entity(to),
            kind: kind.into(),
        }
    }

    fn relation(from: &str, to: &str, kind: RelationKind) -> RelationRow {
        RelationRow {
            from: from.into(),
            to: to.into(),
            kind: kind.as_str().into(),
            created_at: "2026-02-01T00:00:00Z".into(),
        }
    }

    fn fixture() -> (ProjectMap, ProjectGraph, Architecture) {
        let map = ProjectMap {
            as_of: String::new(),
            entities: vec![
                entity("app", EntityKind::Component),
                entity("app-screen", EntityKind::Component),
                entity("app-screen-list", EntityKind::Component),
                entity("store", EntityKind::Component),
                entity("loose", EntityKind::Component),
                entity("sqlite", EntityKind::Technology),
            ],
            part_of: vec![
                ("app-screen".into(), "app".into()),
                ("app-screen-list".into(), "app-screen".into()),
            ],
            pending: 0,
        };
        let graph = ProjectGraph {
            nodes: vec![],
            edges: vec![
                edge(NodeRef::decision("d1"), "app-screen-list", "affects"),
                edge(NodeRef::decision("d1"), "store", "affects"),
                edge(NodeRef::decision("d1"), "sqlite", "uses"),
                edge(NodeRef::decision("d1"), "store", "affects"),
                edge(NodeRef::decision("d2"), "loose", "affects"),
                edge(NodeRef::decision("d2"), "gone", "affects"),
                edge(NodeRef::claim("r1"), "app", "applies_to"),
                edge(NodeRef::entity("app-screen"), "app", "part_of"),
            ],
            suggested: vec![(
                "e9".into(),
                edge(NodeRef::decision("d2"), "store", "affects"),
            )],
        };
        let container = |id: &str| Container {
            entity_id: id.into(),
            name: id.into(),
            ..Container::default()
        };
        let architecture = Architecture {
            containers: vec![container("store"), container("app")],
            interactions: vec![],
            hidden: 0,
        };
        (map, graph, architecture)
    }

    #[test]
    fn decisions_land_on_the_containers_of_what_they_touch() {
        let (map, graph, architecture) = fixture();
        let decisions = vec![
            decision("d1", DecisionStatus::Accepted, "2026-03-01T00:00:00Z"),
            decision("d2", DecisionStatus::Accepted, "2026-04-01T00:00:00Z"),
        ];
        let page = assemble(
            &decisions,
            &BTreeMap::new(),
            &[],
            &map,
            &graph,
            &[],
            &architecture,
        );
        assert_eq!(
            page.decisions
                .iter()
                .map(|d| d.id.as_str())
                .collect::<Vec<_>>(),
            vec!["d2", "d1"],
            "newest first"
        );
        let d1 = &page.decisions[1];
        assert_eq!(d1.label, "D:d1");
        assert_eq!(
            d1.containers,
            vec!["store", "app"],
            "a grandchild climbs to its container; architecture order"
        );
        assert_eq!(
            d1.entities
                .iter()
                .map(|e| (e.id.as_str(), e.edge.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("app-screen-list", "affects"),
                ("store", "affects"),
                ("sqlite", "uses")
            ],
            "duplicate edges count once"
        );
        let d2 = &page.decisions[0];
        assert!(
            d2.containers.is_empty(),
            "a component the architecture does not draw, a retired entity and a pending \
             suggestion give no container"
        );
        assert_eq!(d2.entities.len(), 1);
        assert_eq!(d1.assumptions, vec!["O disco local é confiável"]);
        assert_eq!(d1.reconsider_when, vec!["Se o app virar serviço"]);
        assert!(d1.consequences.is_empty() && d1.scope.is_empty());
        assert_eq!(d1.reason, "o app pode estar fechado");
    }

    #[test]
    fn relations_read_from_each_side_and_superseded_decisions_stay_off() {
        let (map, graph, architecture) = fixture();
        let decisions = vec![
            decision("d1", DecisionStatus::Accepted, "2026-03-01T00:00:00Z"),
            decision("d2", DecisionStatus::Accepted, "2026-04-01T00:00:00Z"),
            decision("d3", DecisionStatus::Accepted, "2026-05-01T00:00:00Z"),
            decision("old", DecisionStatus::Superseded, "2026-01-01T00:00:00Z"),
            decision("stale", DecisionStatus::Accepted, "2026-01-02T00:00:00Z"),
        ];
        let relations = vec![
            relation("d1", "old", RelationKind::Supersedes),
            relation("d3", "stale", RelationKind::Supersedes),
            relation("d2", "d1", RelationKind::DependsOn),
            relation("d3", "d1", RelationKind::ConflictsWith),
            relation("d3", "d1", RelationKind::ConflictsWith),
        ];
        let page = assemble(
            &decisions,
            &BTreeMap::new(),
            &[],
            &map,
            &graph,
            &relations,
            &architecture,
        );
        let ids: Vec<&str> = page.decisions.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["d3", "d2", "d1"],
            "superseded, by status or by relation, is not in force"
        );
        let d1 = page.decisions.iter().find(|d| d.id == "d1").unwrap();
        let seen: Vec<(&str, bool, &str, bool)> = d1
            .relations
            .iter()
            .map(|r| (r.kind.as_str(), r.outgoing, r.other_id.as_str(), r.in_force))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("supersedes", true, "old", false),
                ("depends_on", false, "d2", true),
                ("conflicts_with", true, "d3", true),
            ],
            "a conflict reads the same from both sides and counts once"
        );
        assert_eq!(d1.relations[0].other_title, "Pergunta old?");
        assert_eq!(d1.relations[0].other_label, "D:old");
    }

    #[test]
    fn criteria_keep_the_canonical_order_and_drop_unknown_ones() {
        let (map, graph, architecture) = fixture();
        let decisions = vec![decision(
            "d1",
            DecisionStatus::Accepted,
            "2026-03-01T00:00:00Z",
        )];
        let mut criteria = BTreeMap::new();
        criteria.insert(
            "d1".to_owned(),
            vec![
                "hard_to_reverse".to_owned(),
                "invented".to_owned(),
                "data_or_contract".to_owned(),
            ],
        );
        let page = assemble(&decisions, &criteria, &[], &map, &graph, &[], &architecture);
        assert_eq!(
            page.decisions[0].criteria,
            vec!["data_or_contract", "hard_to_reverse"]
        );
    }

    #[test]
    fn rules_carry_their_label_kind_and_container() {
        let (map, graph, architecture) = fixture();
        let claim = ClaimRecord {
            claim_id: "r1".into(),
            project_id: "p".into(),
            kind: ClaimKind::Constraint,
            statement: "Só uma pessoa confirma.".into(),
            valid_from: "2026-01-01T00:00:00Z".into(),
            valid_until: None,
            source_decision_id: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let page = assemble(
            &[],
            &BTreeMap::new(),
            &[claim],
            &map,
            &graph,
            &[],
            &architecture,
        );
        assert_eq!(page.rules.len(), 1);
        assert_eq!(page.rules[0].label, "R:r1");
        assert_eq!(page.rules[0].kind, "constraint");
        assert_eq!(page.rules[0].containers, vec!["app"]);
    }

    #[test]
    fn the_reason_is_the_first_sentence_on_one_line() {
        assert_eq!(
            first_sentence("Porque o app\n pode estar fechado. Depois."),
            "porque o app pode estar fechado"
        );
        assert_eq!(
            first_sentence("SQLite já está no app."),
            "SQLite já está no app"
        );
        assert_eq!(
            first_sentence("Usa v1.2 do protocolo"),
            "usa v1.2 do protocolo"
        );
        assert_eq!(first_sentence(""), "");
        let long = "a".repeat(400);
        assert_eq!(first_sentence(&long).chars().count(), REASON_CHARS);
    }
}
