//! Several subjects in one provider call.
//!
//! Each AI job kind (rules, relations, links, search terms) used to cost one
//! call per decision. A batch sends up to [`BATCH_SIZE`] subjects of one kind
//! in a numbered list and reads one answer per number; what each subject's
//! answer may store is still checked on its own, exactly as before, so a
//! malformed answer for one subject loses only that subject.

use serde::de::DeserializeOwned;

/// Subjects sent in one provider call, at most.
pub const BATCH_SIZE: usize = 10;

/// Splits subjects into the groups one call may carry: same project, at most
/// [`BATCH_SIZE`] each, in first-seen order. Takes `(position, project)` pairs
/// and answers positions.
pub fn project_groups(subjects: &[(usize, &str)]) -> Vec<Vec<usize>> {
    let mut by_project: Vec<(&str, Vec<usize>)> = Vec::new();
    for (position, project) in subjects {
        match by_project.iter_mut().find(|(known, _)| known == project) {
            Some((_, positions)) => positions.push(*position),
            None => by_project.push((project, vec![*position])),
        }
    }
    by_project
        .into_iter()
        .flat_map(|(_, positions)| {
            positions
                .chunks(BATCH_SIZE)
                .map(<[usize]>::to_vec)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The per-subject entries of a batched answer `{"decisions":[{"id":"1",...}]}`,
/// by position: slot `n - 1` holds the entry whose `id` is `n`. A slot stays
/// `None` when the model skipped that number, repeated it (the first wins) or
/// gave an entry that does not read as `T`; the other slots are unaffected.
/// `None` when the answer is not a JSON object with a `decisions` array.
pub fn answers<T: DeserializeOwned>(answer: &str, subjects: usize) -> Option<Vec<Option<T>>> {
    let value: serde_json::Value = serde_json::from_str(answer.trim()).ok()?;
    let entries = value.get("decisions")?.as_array()?;
    let mut slots: Vec<Option<T>> = (0..subjects).map(|_| None).collect();
    for entry in entries {
        let position = match entry.get("id") {
            Some(serde_json::Value::String(id)) => id.trim().parse::<usize>().ok(),
            Some(serde_json::Value::Number(id)) => id.as_u64().and_then(|id| id.try_into().ok()),
            _ => None,
        };
        let Some(slot) = position
            .and_then(|position| position.checked_sub(1))
            .and_then(|index| slots.get_mut(index))
        else {
            continue;
        };
        if slot.is_none() {
            *slot = serde_json::from_value(entry.clone()).ok();
        }
    }
    Some(slots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct Item {
        #[serde(default)]
        words: Vec<String>,
    }

    #[test]
    fn entries_are_matched_by_number_and_a_bad_one_loses_only_itself() {
        let answer = r#"{"decisions":[
            {"id":"2","words":["b"]},
            {"id":1,"words":["a"]},
            {"id":"3","words":"not a list"},
            {"id":"2","words":["repeated"]},
            {"id":"9","words":["out of range"]},
            {"words":["no id"]}
        ]}"#;
        let slots = answers::<Item>(answer, 4).expect("readable");
        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0].as_ref().unwrap().words, ["a"]);
        assert_eq!(slots[1].as_ref().unwrap().words, ["b"]);
        assert!(slots[2].is_none(), "malformed entry");
        assert!(slots[3].is_none(), "skipped number");
    }

    #[test]
    fn groups_share_a_project_and_hold_at_most_a_batch() {
        let mut subjects: Vec<(usize, &str)> = (0..25).map(|n| (n, "a")).collect();
        subjects.push((25, "b"));
        let groups = project_groups(&subjects);
        let sizes: Vec<usize> = groups.iter().map(Vec::len).collect();
        assert_eq!(sizes, [10, 10, 5, 1]);
        assert_eq!(groups[3], [25]);
    }

    #[test]
    fn an_unreadable_answer_is_none() {
        assert!(answers::<Item>("{", 1).is_none());
        assert!(answers::<Item>(r#"{"other":[]}"#, 1).is_none());
    }
}
