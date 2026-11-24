// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `bench runs`: several scored runs side by side. Each answers file may
//! carry a `run` object recording what the runner knew about that run: a
//! label that groups repeats of one condition, token counts and either the
//! run's own per-million rates or the cost it recorded. Nothing here supplies
//! a price: a run without one gets no cost figure.
//!
//! Per condition it reports the strict pass rate (answers meeting every
//! criterion: the right value on the right authority) beside partial credit
//! (criteria met out of 2 per answer), each repeat's strict passes, the
//! questions that pass on some repeats and not others, and the same counts
//! for each question's `topic` in `questions.json`. Across priced conditions
//! it marks the accuracy against cost frontier, and across 2 or more
//! conditions it lists the questions every condition passes or none does.

use num_bigint::BigInt;

use super::{
    Cited, EXIT_OK, Key, Scored, Value, decimal_field, family_json, load_keys, once_each,
    rate_lines, rates_json, ratio, read_answers, score, text,
};
use crate::evals::json_int;
use crate::gate::{self, Q};
use crate::py::PyPath;
use crate::pyjson::{self, Json};

const RUN_FIELDS: [&str; 6] = [
    "model",
    "input_tokens",
    "output_tokens",
    "input_per_million",
    "output_per_million",
    "cost_usd",
];

/// The slice name for a question whose `questions.json` entry has no `topic`.
pub const NO_FAMILY: &str = "(no topic)";

/// What one answers file's `run` object records.
#[derive(Debug, Default)]
pub struct RunRecord {
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    cost: Option<Q>,
}

impl RunRecord {
    /// The run's cost in USD to 4 places, when the record prices it.
    pub fn cost_usd(&self) -> Option<String> {
        self.cost.as_ref().map(|c| gate::decimal(c, 4))
    }
}

/// Read an answers file's optional `run` object. Rates need both token
/// counts; a recorded `cost_usd` stands alone and cannot sit beside rates.
pub fn read_run(answers: &Json) -> Result<Option<RunRecord>, String> {
    let Some(run) = answers.get("run") else {
        return Ok(None);
    };
    let at = "answers file: run";
    let Json::Object(fields) = run else {
        return Err(format!("{at} must be an object"));
    };
    if let Some((field, _)) = fields
        .iter()
        .find(|(f, _)| !RUN_FIELDS.contains(&f.as_str()))
    {
        return Err(format!("{at}: unknown field {field}"));
    }
    let model = match run.get("model") {
        None => None,
        Some(_) => match text(run, "model", at)?.trim() {
            "" => return Err(format!("{at}: model must not be empty")),
            name => Some(name.to_string()),
        },
    };
    let tokens = |field: &str| {
        run.get(field)
            .map(|v| {
                json_int(v)
                    .and_then(|n| u64::try_from(n).ok())
                    .ok_or_else(|| format!("{at}: {field} must be a whole number"))
            })
            .transpose()
    };
    let money = |field: &str| {
        run.get(field)
            .map(|_| {
                let written = text(run, field, at)?;
                if written.trim_end().ends_with('%') {
                    return Err(format!("{at}: {field} {written} is not a number"));
                }
                let q = decimal_field(run, field, at)?;
                if q.less_than(&ratio(0, 1)) {
                    return Err(format!("{at}: {field} must not be negative"));
                }
                Ok(q)
            })
            .transpose()
    };
    let (input_tokens, output_tokens) = (tokens("input_tokens")?, tokens("output_tokens")?);
    let rates = (money("input_per_million")?, money("output_per_million")?);
    let cost = match (money("cost_usd")?, rates) {
        (Some(_), (Some(_), _) | (_, Some(_))) => {
            return Err(format!(
                "{at}: give cost_usd or the per-million rates, not both"
            ));
        }
        (recorded, (None, None)) => recorded,
        (None, (Some(input), Some(output))) => {
            let (Some(i), Some(o)) = (input_tokens, output_tokens) else {
                return Err(format!(
                    "{at}: per-million rates need input_tokens and output_tokens"
                ));
            };
            let per = |t: u64, rate: &Q| Q::new(BigInt::from(t), BigInt::from(1_000_000)).mul(rate);
            Some(per(i, &input).add(&per(o, &output), 1))
        }
        (None, _) => {
            return Err(format!(
                "{at}: give both input_per_million and output_per_million"
            ));
        }
    };
    Ok(Some(RunRecord {
        model,
        input_tokens,
        output_tokens,
        cost,
    }))
}

/// Every criterion the scorer applies: the right value on the right authority.
pub fn strict(s: &Scored) -> bool {
    s.value == Value::Correct && s.cited == Cited::Right
}

/// Criteria met, out of 2: the value, and the authority.
pub fn criteria(s: &Scored) -> usize {
    usize::from(s.value == Value::Correct) + usize::from(s.cited == Cited::Right)
}

fn frac(n: usize, d: usize) -> Q {
    Q::new(BigInt::from(n), BigInt::from(d))
}

/// `n` of `d` as a decimal to 4 places, and `0.0000` when `d` is 0.
pub fn share(n: usize, d: usize) -> String {
    gate::decimal(&frac(n, d.max(1)), 4)
}

/// Counts for one `topic` across every answer to its questions.
#[derive(Debug, PartialEq)]
pub struct Slice {
    pub name: String,
    pub questions: usize,
    pub answers: usize,
    pub strict_passed: usize,
    pub criteria_met: usize,
}

/// Slice repeats of one condition by each question's `topic`, in the order
/// the topics first appear, naming questions without one `NO_FAMILY`.
pub fn families(keys: &[Key], runs: &[&[Scored]]) -> Vec<Slice> {
    let mut found: Vec<Slice> = Vec::new();
    for (q, key) in keys.iter().enumerate() {
        let name = key.family.as_deref().unwrap_or(NO_FAMILY);
        let at = match found.iter().position(|f| f.name == name) {
            Some(at) => at,
            None => {
                found.push(Slice {
                    name: name.to_string(),
                    questions: 0,
                    answers: 0,
                    strict_passed: 0,
                    criteria_met: 0,
                });
                found.len() - 1
            }
        };
        let family = &mut found[at];
        family.questions += 1;
        family.answers += runs.len();
        family.strict_passed += runs.iter().filter(|r| strict(&r[q])).count();
        family.criteria_met += runs.iter().map(|r| criteria(&r[q])).sum::<usize>();
    }
    found
}

/// A question that met every criterion on some repeats and not others.
#[derive(Debug, PartialEq)]
pub struct Unstable {
    pub id: String,
    pub strict_passed: usize,
    pub repeats: usize,
}

/// One condition: the answers files sharing a `run.model` label, or one
/// unlabelled file.
#[derive(Debug, PartialEq)]
pub struct Condition {
    pub label: String,
    pub files: Vec<String>,
    pub questions: usize,
    pub strict_per_repeat: Vec<usize>,
    pub strict_passed: usize,
    pub strict_rate: String,
    pub criteria_met: usize,
    pub partial_credit: String,
    pub families: Vec<Slice>,
    pub unstable: Vec<Unstable>,
    /// Totals, when every file records both counts.
    pub tokens: Option<(u64, u64)>,
    pub priced_files: usize,
    /// Set only when every file is priced.
    pub cost_usd: Option<String>,
    pub mean_cost_per_run_usd: Option<String>,
    /// Absent when unpriced or when no answer passed.
    pub cost_per_passed_task_usd: Option<String>,
    /// `None` when unpriced.
    pub on_frontier: Option<bool>,
}

/// Questions that no longer tell the conditions apart.
#[derive(Debug, PartialEq)]
pub struct Saturation {
    pub every_condition_passes: Vec<String>,
    pub every_condition_fails: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct RunsReport {
    pub conditions: Vec<Condition>,
    /// `None` with fewer than 2 conditions.
    pub saturation: Option<Saturation>,
}

struct Group<'a> {
    label: String,
    by_model: bool,
    files: Vec<&'a str>,
    scored: Vec<Vec<Scored>>,
    records: Vec<Option<RunRecord>>,
}

fn condition(keys: &[Key], g: &Group<'_>) -> (Condition, Option<(Q, Q)>) {
    let n = g.scored.len();
    let answers = keys.len() * n;
    let strict_per_repeat: Vec<usize> = g
        .scored
        .iter()
        .map(|r| r.iter().filter(|s| strict(s)).count())
        .collect();
    let strict_passed: usize = strict_per_repeat.iter().sum();
    let criteria_met: usize = g.scored.iter().flatten().map(criteria).sum();
    let unstable = keys
        .iter()
        .enumerate()
        .filter_map(|(q, key)| {
            let k = g.scored.iter().filter(|r| strict(&r[q])).count();
            (k > 0 && k < n).then(|| Unstable {
                id: key.id.clone(),
                strict_passed: k,
                repeats: n,
            })
        })
        .collect();
    let records: Vec<&RunRecord> = g.records.iter().flatten().collect();
    let tokens = if records.len() == n {
        records.iter().try_fold((0u64, 0u64), |(i, o), r| {
            Some((
                i.checked_add(r.input_tokens?)?,
                o.checked_add(r.output_tokens?)?,
            ))
        })
    } else {
        None
    };
    let costs: Vec<&Q> = records.iter().filter_map(|r| r.cost.as_ref()).collect();
    let total = (costs.len() == n).then(|| costs.iter().fold(ratio(0, 1), |t, c| t.add(c, 1)));
    let mean = total.as_ref().map(|t| t.mul(&frac(1, n)));
    let usd = |q: &Q| gate::decimal(q, 4);
    let per_passed = total.as_ref().and_then(|t| t.div(&frac(strict_passed, 1)));
    let point = mean.clone().map(|m| (frac(strict_passed, answers), m));
    (
        Condition {
            label: g.label.clone(),
            files: g.files.iter().map(|f| f.to_string()).collect(),
            questions: keys.len(),
            strict_rate: share(strict_passed, answers),
            strict_per_repeat,
            strict_passed,
            partial_credit: share(criteria_met, 2 * answers),
            criteria_met,
            families: families(
                keys,
                &g.scored.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            ),
            unstable,
            tokens,
            priced_files: costs.len(),
            cost_usd: total.as_ref().map(usd),
            mean_cost_per_run_usd: mean.as_ref().map(usd),
            cost_per_passed_task_usd: per_passed.as_ref().map(usd),
            on_frontier: None,
        },
        point,
    )
}

/// Whether `b` is at least as accurate and at most as costly as `a`, and
/// better on one of the 2.
fn dominates(b: &(Q, Q), a: &(Q, Q)) -> bool {
    let no_worse = !b.0.less_than(&a.0) && !a.1.less_than(&b.1);
    no_worse && (a.0.less_than(&b.0) || b.1.less_than(&a.1))
}

/// Score each `(path, answers)` file, group them by `run.model` (a file
/// without one is its own condition, labelled by its path) in the order they
/// first appear, and report each condition, the frontier and saturation.
pub fn summarise(keys: &[Key], files: &[(String, Json)]) -> Result<RunsReport, String> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for (path, answers) in files {
        let scored = score(keys, answers).map_err(|e| format!("{e} ({path})"))?;
        let record = read_run(answers).map_err(|e| format!("{e} ({path})"))?;
        let model = record.as_ref().and_then(|r| r.model.clone());
        // Only a recorded model merges files: a path-labelled file stays its
        // own condition even when another file's model has the same text.
        let by_model = model.is_some();
        let label = model.unwrap_or_else(|| path.clone());
        match groups
            .iter_mut()
            .find(|g| by_model && g.by_model && g.label == label)
        {
            Some(g) => {
                g.files.push(path.as_str());
                g.scored.push(scored);
                g.records.push(record);
            }
            None => groups.push(Group {
                label,
                by_model,
                files: vec![path.as_str()],
                scored: vec![scored],
                records: vec![record],
            }),
        }
    }
    let (mut conditions, points): (Vec<Condition>, Vec<Option<(Q, Q)>>) =
        groups.iter().map(|g| condition(keys, g)).unzip();
    for (c, point) in conditions.iter_mut().zip(&points) {
        c.on_frontier = point
            .as_ref()
            .map(|p| !points.iter().flatten().any(|other| dominates(other, p)));
    }
    let saturation = (groups.len() > 1).then(|| {
        let ids = |held: &dyn Fn(usize) -> bool| -> Vec<String> {
            (0..keys.len())
                .filter(|&q| held(q))
                .map(|q| keys[q].id.clone())
                .collect()
        };
        let answers = |q: usize| {
            groups
                .iter()
                .flat_map(move |g| g.scored.iter().map(move |r| &r[q]))
        };
        Saturation {
            every_condition_passes: ids(&|q| answers(q).all(strict)),
            every_condition_fails: ids(&|q| !answers(q).any(strict)),
        }
    });
    Ok(RunsReport {
        conditions,
        saturation,
    })
}

fn runs_json(report: &RunsReport) -> Json {
    let int = |n: usize| Json::Int(BigInt::from(n));
    let usd = |v: &Option<String>| v.as_deref().map_or(Json::Null, Json::str_obj);
    let ids = |list: &[String]| Json::Array(list.iter().map(|id| Json::str_obj(id)).collect());
    let unstable = |c: &Condition| {
        let rows = c.unstable.iter().map(|u| {
            Json::Object(vec![
                ("id".into(), Json::str_obj(&u.id)),
                ("strict_passed".into(), int(u.strict_passed)),
                ("repeats".into(), int(u.repeats)),
            ])
        });
        Json::Array(rows.collect())
    };
    let tokens = |c: &Condition| {
        c.tokens.map_or(Json::Null, |(i, o)| {
            Json::Object(vec![
                ("input".into(), Json::Int(BigInt::from(i))),
                ("output".into(), Json::Int(BigInt::from(o))),
            ])
        })
    };
    let conditions = report.conditions.iter().map(|c| {
        let per_repeat = c.strict_per_repeat.iter().map(|&n| int(n)).collect();
        let mut fields: Vec<(String, Json)> = vec![
            ("label".into(), Json::str_obj(&c.label)),
            ("files".into(), ids(&c.files)),
            ("repeats".into(), int(c.files.len())),
            ("questions".into(), int(c.questions)),
            ("strict_per_repeat".into(), Json::Array(per_repeat)),
        ];
        fields.extend(rates_json(
            c.questions * c.files.len(),
            c.strict_passed,
            c.criteria_met,
        ));
        fields.extend([
            ("families".into(), family_json(&c.families)),
            ("unstable".into(), unstable(c)),
            ("tokens".into(), tokens(c)),
            ("priced_files".into(), int(c.priced_files)),
            ("cost_usd".into(), usd(&c.cost_usd)),
            (
                "mean_cost_per_run_usd".into(),
                usd(&c.mean_cost_per_run_usd),
            ),
            (
                "cost_per_passed_task_usd".into(),
                usd(&c.cost_per_passed_task_usd),
            ),
            (
                "on_frontier".into(),
                c.on_frontier.map_or(Json::Null, Json::Bool),
            ),
        ]);
        Json::Object(fields)
    });
    let saturation = report.saturation.as_ref().map_or(Json::Null, |s| {
        Json::Object(vec![
            (
                "every_condition_passes".into(),
                ids(&s.every_condition_passes),
            ),
            (
                "every_condition_fails".into(),
                ids(&s.every_condition_fails),
            ),
        ])
    });
    Json::Object(vec![
        ("conditions".into(), Json::Array(conditions.collect())),
        ("saturation".into(), saturation),
    ])
}

fn listed(items: &[String]) -> String {
    if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    }
}

fn condition_lines(c: &Condition) {
    let repeats = c.files.len();
    outln!("condition {}: {repeats} answers file(s)", c.label);
    rate_lines(
        c.questions * repeats,
        c.strict_passed,
        c.criteria_met,
        &c.families,
    );
    let per_repeat: Vec<String> = c.strict_per_repeat.iter().map(usize::to_string).collect();
    outln!("strict passes per repeat: {}", per_repeat.join(", "));
    if repeats < 2 {
        outln!("unstable: needs 2 or more repeats");
    } else {
        let unstable: Vec<String> = c
            .unstable
            .iter()
            .map(|u| format!("{} {}/{}", u.id, u.strict_passed, u.repeats))
            .collect();
        outln!("unstable: {}", listed(&unstable));
    }
    match c.tokens {
        Some((i, o)) => outln!("tokens: {i} input, {o} output"),
        None => outln!("tokens: not recorded on every file"),
    }
    match (&c.cost_usd, &c.mean_cost_per_run_usd, c.on_frontier) {
        (Some(total), Some(mean), Some(frontier)) => {
            let per_passed = c
                .cost_per_passed_task_usd
                .as_ref()
                .map_or_else(|| "none passed".to_string(), |p| format!("${p} USD"));
            let place = if frontier {
                "on the frontier"
            } else {
                "below the frontier"
            };
            outln!("cost: ${total} USD, ${mean} USD a run; per passed task: {per_passed}; {place}");
        }
        _ => outln!(
            "cost: not priced ({} of {repeats} file(s) record a cost)",
            c.priced_files
        ),
    }
}

/// `bench runs`: score every file, then print or emit the report.
pub(super) fn run_runs(
    root: &PyPath,
    topic: &str,
    paths: &[String],
    json: bool,
) -> Result<u8, String> {
    let keys = load_keys(root, topic)?;
    once_each(paths.iter())?;
    let files = paths
        .iter()
        .map(|p| {
            read_answers(p)
                .map(|a| (p.clone(), a))
                .map_err(|e| format!("{e} ({p})"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let report = summarise(&keys, &files)?;
    if json {
        outln!("{}", pyjson::dumps(&runs_json(&report), false));
        return Ok(EXIT_OK);
    }
    outln!(
        "bench runs: {} answers file(s), {} condition(s), {} questions (topic {topic})",
        paths.len(),
        report.conditions.len(),
        keys.len()
    );
    for c in &report.conditions {
        condition_lines(c);
    }
    match &report.saturation {
        None => outln!("saturation: needs 2 or more conditions"),
        Some(s) => {
            outln!(
                "every condition passes: {}",
                listed(&s.every_condition_passes)
            );
            outln!(
                "every condition fails: {}",
                listed(&s.every_condition_fails)
            );
        }
    }
    Ok(EXIT_OK)
}
