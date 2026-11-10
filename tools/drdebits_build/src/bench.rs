// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! The Payday Super and Division 7A benchmark: published questions, documented
//! keys and a scorer. Nothing here calls a model.
//!
//! `bench keys` recomputes each key that follows from a rule, from the inputs
//! the key records and a published table of non-business days. It does not read
//! the sources of the other keys. It takes each engine value from the recorded
//! output and compares it with the key. `bench score` marks a file of
//! answers on 2 separate axes, because a right figure can rest on the wrong
//! section: the value, and the authority cited for it. Authorities compare at
//! section level, so `s 109E(6)` meets a key naming `s 109E`, and each section
//! a key lists is enough alone. A right section beside a readable wrong one is
//! mixed, so citing every section earns nothing. A right section beside a form
//! the gate cannot read is partly unreadable, because that form could be wrong.
//! A Division, bare Act name or other unreadable form alone counts as
//! unreadable, not as wrong.
//!
//! A key is what its documented source and the published derivation give. It
//! is not advice, and a score is not a measure of anything outside these
//! questions. See `benchmarks/payday-div7a/README.md`.

use std::collections::HashSet;

use num_bigint::BigInt;

use crate::builder;
use crate::evals::json_int;
use crate::gate::{self, Authority, Q};
use crate::py::{Date, PyPath};
use crate::pyjson::{self, Json};
use crate::templates::BENCHMARK_CANARY;

pub const QUESTIONS_FILE: &str = "benchmarks/payday-div7a/questions.json";
pub const KEYS_FILE: &str = "benchmarks/payday-div7a/keys.json";
pub const CALENDAR_FILE: &str = "benchmarks/payday-div7a/business-days.json";
pub const ENGINE_FILE: &str = "benchmarks/payday-div7a/engine-check.json";

/// What `keys.json` must say about several listed authorities, because the
/// scorer applies exactly this.
const AUTHORITY_RULE: &str = "Each section listed under a key is enough alone. For Payday Super keys either s 6 or s 18C counts as right, because each states the period relied on.";

pub const EXIT_OK: u8 = 0;
pub const EXIT_MISMATCH: u8 = 1;
pub const EXIT_COULD_NOT_RUN: u8 = 2;

/// How a key's value is compared with an answer.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// A date or fixed token, compared as text, ignoring case.
    Text,
    /// An amount, compared by value; `$`, commas and a trailing `%` are ignored.
    Number,
}

/// How a key's value can be reproduced offline.
#[derive(Debug)]
enum Rule {
    /// Read from the named source; there is nothing to compute.
    Source,
    /// The latest of `(from, days)` business-day counts, each excluding `from`.
    BusinessDays(Vec<(Date, i64)>),
    /// The s 109E(6) formula, rounded half up to `places`.
    Myr {
        principal: Q,
        rate: Q,
        years: u32,
        places: u32,
    },
    /// The s 109E(6) formula rounded to cents, less the amount `paid`.
    Shortfall {
        principal: Q,
        rate: Q,
        years: u32,
        paid: Q,
    },
}

#[derive(Debug)]
pub struct Key {
    pub id: String,
    pub value: String,
    kind: Kind,
    authorities: Vec<Authority>,
    rule: Rule,
}

fn text<'a>(item: &'a Json, field: &str, at: &str) -> Result<&'a str, String> {
    item.get(field)
        .and_then(Json::as_str)
        .ok_or_else(|| format!("{at}: {field} must be text"))
}

fn items<'a>(payload: &'a Json, field: &str, at: &str) -> Result<&'a [Json], String> {
    match payload.get(field) {
        Some(Json::Array(items)) if !items.is_empty() => Ok(items),
        _ => Err(format!("{at}: {field} must be a list with entries")),
    }
}

fn date(value: &str, at: &str) -> Result<Date, String> {
    Date::fromisoformat(value).ok_or_else(|| format!("{at}: {value} is not a date"))
}

fn amount(value: &str) -> Option<Q> {
    let written = value.trim();
    gate::number(written.strip_suffix('%').unwrap_or(written)).map(|(q, ..)| q)
}

fn decimal_field(item: &Json, field: &str, at: &str) -> Result<Q, String> {
    let written = text(item, field, at)?;
    amount(written).ok_or_else(|| format!("{at}: {field} {written} is not a number"))
}

fn whole(item: &Json, field: &str, at: &str) -> Result<u32, String> {
    item.get(field)
        .and_then(json_int)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("{at}: {field} must be a whole number"))
}

fn read_rule(check: &Json, at: &str) -> Result<Rule, String> {
    let rule = text(check, "rule", at)?;
    match rule {
        "source" => Ok(Rule::Source),
        "business-days" => items(check, "terms", at)?
            .iter()
            .map(|term| {
                let days = whole(term, "days", at)?;
                Ok((date(text(term, "from", at)?, at)?, i64::from(days)))
            })
            .collect::<Result<_, String>>()
            .map(Rule::BusinessDays),
        "myr" => Ok(Rule::Myr {
            principal: decimal_field(check, "principal", at)?,
            rate: decimal_field(check, "rate", at)?,
            years: whole(check, "years", at)?,
            places: whole(check, "places", at)?,
        }),
        "shortfall" => Ok(Rule::Shortfall {
            principal: decimal_field(check, "principal", at)?,
            rate: decimal_field(check, "rate", at)?,
            years: whole(check, "years", at)?,
            paid: decimal_field(check, "paid", at)?,
        }),
        other => Err(format!("{at}: unknown rule {other}")),
    }
}

fn read_key(item: &Json) -> Result<Key, String> {
    let id = text(item, "id", "keys file")?.to_string();
    let at = format!("keys file {id}");
    let kind = match text(item, "kind", &at)? {
        "text" => Kind::Text,
        "number" => Kind::Number,
        other => return Err(format!("{at}: unknown kind {other}")),
    };
    let value = text(item, "value", &at)?.to_string();
    if kind == Kind::Number && amount(&value).is_none() {
        return Err(format!("{at}: value {value} is not a number"));
    }
    let authorities = items(item, "authorities", &at)?
        .iter()
        .map(|a| {
            let written = a.as_str().unwrap_or_default();
            Authority::read(written).ok_or_else(|| format!("{at}: cannot read {written}"))
        })
        .collect::<Result<_, String>>()?;
    for field in ["source", "derivation"] {
        if item.get(field).is_none() {
            return Err(format!("{at}: {field} is missing"));
        }
    }
    let check = item.get("check").ok_or(format!("{at}: check is missing"))?;
    Ok(Key {
        kind,
        authorities,
        rule: read_rule(check, &at)?,
        id,
        value,
    })
}

fn read_json(root: &PyPath, rel: &str) -> Result<Json, String> {
    let body = builder::read_text(&root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    pyjson::loads(&body).map_err(|e| format!("{rel}: {e}"))
}

fn with_canary(payload: Json, rel: &str) -> Result<Json, String> {
    match payload.get("benchmark_canary").and_then(Json::as_str) {
        Some(BENCHMARK_CANARY) => Ok(payload),
        _ => Err(format!("{rel}: benchmark_canary is missing or changed")),
    }
}

/// Read the keys, refusing a set that does not answer the questions one for one.
pub fn load_keys(root: &PyPath) -> Result<Vec<Key>, String> {
    let questions = with_canary(read_json(root, QUESTIONS_FILE)?, QUESTIONS_FILE)?;
    let keys = with_canary(read_json(root, KEYS_FILE)?, KEYS_FILE)?;
    if keys.get("authority_rule").and_then(Json::as_str) != Some(AUTHORITY_RULE) {
        return Err(format!("{KEYS_FILE}: authority_rule is missing or changed"));
    }
    let asked = items(&questions, "questions", QUESTIONS_FILE)?
        .iter()
        .map(|q| text(q, "id", QUESTIONS_FILE))
        .collect::<Result<Vec<_>, _>>()?;
    let keys = items(&keys, "keys", KEYS_FILE)?
        .iter()
        .map(read_key)
        .collect::<Result<Vec<_>, _>>()?;
    let keyed: Vec<&str> = keys.iter().map(|k| k.id.as_str()).collect();
    if asked != keyed {
        return Err("the keys do not match the questions, id for id and in order".into());
    }
    if keyed.iter().collect::<HashSet<_>>().len() != keyed.len() {
        return Err("a question id appears twice".into());
    }
    Ok(keys)
}

/// The table of non-business weekdays and the dates it is complete between.
struct Calendar {
    closed: HashSet<Date>,
    from: Date,
    until: Date,
}

fn load_calendar(root: &PyPath) -> Result<Calendar, String> {
    let payload = read_json(root, CALENDAR_FILE)?;
    let closed = items(&payload, "non_business_days", CALENDAR_FILE)?
        .iter()
        .map(|d| date(text(d, "date", CALENDAR_FILE)?, CALENDAR_FILE))
        .collect::<Result<_, _>>()?;
    let bound = |field| date(text(&payload, field, CALENDAR_FILE)?, CALENDAR_FILE);
    Ok(Calendar {
        closed,
        from: bound("complete_from")?,
        until: bound("complete_until")?,
    })
}

/// `from` plus `days` business days, `from` itself never counted. A weekend is
/// closed by its ordinal, as `date.weekday()` numbers it.
fn add_business_days(from: Date, days: i64, closed: &HashSet<Date>) -> Option<Date> {
    let (mut ordinal, mut left) = (from.ordinal(), days);
    while left > 0 {
        ordinal += 1;
        let day = Date::from_ordinal(ordinal)?;
        if (ordinal + 6) % 7 < 5 && !closed.contains(&day) {
            left -= 1;
        }
    }
    Date::from_ordinal(ordinal)
}

/// `P × r ÷ (1 − (1 ÷ (1 + r))^t)`, exactly.
fn myr(principal: &Q, rate: &Q, years: u32) -> Option<Q> {
    let one = Q::new(BigInt::from(1), BigInt::from(1));
    let growth = one.add(rate, 1);
    let factor = (0..years).fold(one.clone(), |power, _| power.mul(&growth));
    principal.mul(rate).div(&one.add(&one.div(&factor)?, -1))
}

/// The value a key's rule gives, or `None` for a key not recomputed here.
fn reproduce(key: &Key, calendar: &Calendar) -> Result<Option<String>, String> {
    let fail = || format!("{}: the rule cannot be worked", key.id);
    Ok(match &key.rule {
        Rule::Source => None,
        Rule::BusinessDays(terms) => {
            let mut latest = None;
            for (from, days) in terms {
                if *from < calendar.from {
                    return Err(format!(
                        "{}: {from:?} is before the calendar's first complete day",
                        key.id
                    ));
                }
                let due = add_business_days(*from, *days, &calendar.closed).ok_or_else(fail)?;
                if due > calendar.until {
                    return Err(format!(
                        "{}: {due:?} is past the calendar's last complete day",
                        key.id
                    ));
                }
                latest = latest.max(Some(due));
            }
            latest.map(|d| d.isoformat())
        }
        Rule::Myr {
            principal,
            rate,
            years,
            places,
        } => Some(gate::decimal(
            &myr(principal, rate, *years).ok_or_else(fail)?,
            *places,
        )),
        Rule::Shortfall {
            principal,
            rate,
            years,
            paid,
        } => {
            let required = gate::decimal(&myr(principal, rate, *years).ok_or_else(fail)?, 2);
            let required = amount(&required).ok_or_else(fail)?;
            Some(gate::decimal(&required.add(paid, -1), 2))
        }
    })
}

fn matches(kind: Kind, key: &str, answer: &str) -> bool {
    match kind {
        Kind::Text => key.eq_ignore_ascii_case(answer.trim()),
        Kind::Number => match (amount(key), amount(answer)) {
            (Some(a), Some(b)) => a.equals(&b),
            _ => false,
        },
    }
}

/// What `bench keys` found.
#[derive(Debug, PartialEq)]
pub struct KeyReport {
    pub computed: usize,
    /// Keys with no rule. Their sources are not read here.
    pub not_recomputed: usize,
    pub engine_checked: usize,
    pub problems: Vec<String>,
}

/// The value at `path` in a recorded engine output, in the key's form. The
/// record's own `value` plays no part, so it cannot stand in for the output.
fn engine_value(check: &Json, at: &str) -> Result<String, String> {
    let output = check
        .get("output")
        .ok_or(format!("{at}: output is missing"))?;
    let read = check.get("read").ok_or(format!("{at}: read is missing"))?;
    let Some(Json::Array(path)) = read.get("path") else {
        return Err(format!("{at}: read path must be a list"));
    };
    let found = path
        .iter()
        .try_fold(output, |here, step| match (here, step) {
            (_, Json::Str(field)) => here.get(field),
            (Json::Array(items), step) => {
                items.get(json_int(step).and_then(|i| usize::try_from(i).ok())?)
            }
            _ => None,
        })
        .ok_or(format!("{at}: nothing at the read path"))?;
    let number = || {
        found
            .as_str()
            .and_then(amount)
            .ok_or(format!("{at}: the output at the read path is not a number"))
    };
    match text(read, "form", at)? {
        "text" => found
            .as_str()
            .map(str::to_string)
            .ok_or(format!("{at}: the output at the read path is not text")),
        "percent" => Ok(gate::decimal(
            &number()?.mul(&Q::new(BigInt::from(100), BigInt::from(1))),
            2,
        )),
        "whole-dollars" => Ok(gate::decimal(&number()?, 0)),
        "none-for-year-of-loan" => {
            let refused = matches!(found, Json::Array(codes)
                if codes.iter().any(|c| c.as_str() == Some("REFUSED_YEAR_IS_YEAR_OF_LOAN")));
            if refused && matches!(output.get("myr_required"), Some(Json::Null)) {
                Ok("NONE".into())
            } else {
                Err(format!(
                    "{at}: the output does not refuse the year of the loan"
                ))
            }
        }
        other => Err(format!("{at}: unknown read form {other}")),
    }
}

/// Recompute what can be recomputed and compare every key with the value taken
/// from the recorded engine output. A duplicate record stops the check.
pub fn check_keys(root: &PyPath) -> Result<KeyReport, String> {
    let keys = load_keys(root)?;
    let calendar = load_calendar(root)?;
    let engine = read_json(root, ENGINE_FILE)?;
    let checks = items(&engine, "checks", ENGINE_FILE)?;
    let mut ids = HashSet::new();
    for check in checks {
        let id = text(check, "id", ENGINE_FILE)?;
        if !ids.insert(id) {
            return Err(format!("{ENGINE_FILE}: {id} is recorded twice"));
        }
    }
    let mut report = KeyReport {
        computed: 0,
        not_recomputed: 0,
        engine_checked: 0,
        problems: Vec::new(),
    };
    for key in &keys {
        match reproduce(key, &calendar)? {
            None => report.not_recomputed += 1,
            Some(value) => {
                report.computed += 1;
                if !matches(key.kind, &key.value, &value) {
                    report.problems.push(format!(
                        "{}: the key says {} and the rule gives {value}",
                        key.id, key.value
                    ));
                }
            }
        }
        let recorded = checks
            .iter()
            .find(|c| c.get("id").and_then(Json::as_str) == Some(key.id.as_str()));
        let Some(check) = recorded else {
            report
                .problems
                .push(format!("{}: no engine output is recorded", key.id));
            continue;
        };
        let at = format!("{ENGINE_FILE} {}", key.id);
        let value = engine_value(check, &at)?;
        let stated = text(check, "value", &at)?;
        if !matches(key.kind, stated, &value) {
            report.problems.push(format!(
                "{}: the record states {stated} but its output gives {value}",
                key.id
            ));
        }
        if matches(key.kind, &key.value, &value) {
            report.engine_checked += 1;
        } else {
            report.problems.push(format!(
                "{}: the key says {} and the engine gave {value}",
                key.id, key.value
            ));
        }
    }
    Ok(report)
}

/// The value axis of one answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Correct,
    Wrong,
    Unanswered,
}

/// The authority axis of one answer: every citation is readable and names a
/// listed section, or some readable ones do and some do not, or those that can
/// be read all do beside one that cannot, or none does, or nothing cited can be
/// read, or nothing was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cited {
    Right,
    Mixed,
    PartlyUnreadable,
    Wrong,
    Unreadable,
    Missing,
}

#[derive(Debug, PartialEq)]
pub struct Scored {
    pub id: String,
    pub value: Value,
    pub cited: Cited,
}

fn cited(key: &Key, written: &[&str]) -> Cited {
    let read: Vec<Authority> = written.iter().filter_map(|c| Authority::read(c)).collect();
    let right = read
        .iter()
        .filter(|a| key.authorities.iter().any(|k| k.section() == a.section()))
        .count();
    if written.is_empty() {
        Cited::Missing
    } else if read.is_empty() {
        Cited::Unreadable
    } else if right == 0 {
        Cited::Wrong
    } else if right < read.len() {
        Cited::Mixed
    } else if read.len() < written.len() {
        Cited::PartlyUnreadable
    } else {
        Cited::Right
    }
}

/// Mark an answers file, refusing one that names an unknown id or field.
pub fn score(keys: &[Key], answers: &Json) -> Result<Vec<Scored>, String> {
    let fail = |m: String| format!("answers file: {m}");
    let missing_answers = || fail("the 'answers' key must hold an object".into());
    let Json::Object(top) = answers else {
        return Err(missing_answers());
    };
    let Some((_, Json::Object(given))) = top.iter().find(|(field, _)| field == "answers") else {
        return Err(missing_answers());
    };
    if let Some((field, _)) = top.iter().find(|(field, _)| field != "answers") {
        return Err(fail(format!("unknown field {field}")));
    }
    if let Some((id, _)) = given
        .iter()
        .find(|(id, _)| !keys.iter().any(|k| &k.id == id))
    {
        return Err(fail(format!("unknown question {id}")));
    }
    let mut scored = Vec::new();
    for key in keys {
        let Some(answer) = answers.get("answers").and_then(|a| a.get(&key.id)) else {
            scored.push(Scored {
                id: key.id.clone(),
                value: Value::Unanswered,
                cited: Cited::Missing,
            });
            continue;
        };
        let Json::Object(fields) = answer else {
            return Err(fail(format!("{} must be an object", key.id)));
        };
        if let Some((field, _)) = fields
            .iter()
            .find(|(f, _)| f != "value" && f != "authorities")
        {
            return Err(fail(format!("{}: unknown field {field}", key.id)));
        }
        let value = text(answer, "value", &fail(key.id.clone()))?;
        let written = match answer.get("authorities") {
            None => Vec::new(),
            Some(Json::Array(list)) => list
                .iter()
                .map(|a| {
                    a.as_str()
                        .ok_or(fail(format!("{}: authorities must be text", key.id)))
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(fail(format!("{}: authorities must be a list", key.id))),
        };
        scored.push(Scored {
            id: key.id.clone(),
            value: if value.trim().is_empty() {
                Value::Unanswered
            } else if matches(key.kind, &key.value, value) {
                Value::Correct
            } else {
                Value::Wrong
            },
            cited: cited(key, &written),
        });
    }
    Ok(scored)
}

fn tally(scored: &[Scored]) -> Vec<(&'static str, usize)> {
    let values = |v: Value| scored.iter().filter(|s| s.value == v).count();
    let answered = |c: Cited| {
        scored
            .iter()
            .filter(|s| s.value != Value::Unanswered && s.cited == c)
            .count()
    };
    vec![
        ("cases", scored.len()),
        ("value_correct", values(Value::Correct)),
        ("value_wrong", values(Value::Wrong)),
        ("unanswered", values(Value::Unanswered)),
        ("authority_right", answered(Cited::Right)),
        ("authority_mixed", answered(Cited::Mixed)),
        (
            "authority_partly_unreadable",
            answered(Cited::PartlyUnreadable),
        ),
        ("wrong_authority", answered(Cited::Wrong)),
        ("authority_unreadable", answered(Cited::Unreadable)),
        ("authority_missing", answered(Cited::Missing)),
        (
            "correct_value_wrong_authority",
            scored
                .iter()
                .filter(|s| s.value == Value::Correct && s.cited == Cited::Wrong)
                .count(),
        ),
        (
            "correct_value_right_authority",
            scored
                .iter()
                .filter(|s| s.value == Value::Correct && s.cited == Cited::Right)
                .count(),
        ),
    ]
}

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Recompute the keys and compare them with the recorded engine output.
    Keys,
    /// Mark a JSON answers file on value and authority.
    Score {
        /// The answers file.
        answers: String,
        /// Print each question's marks and the totals as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn run_keys(root: &PyPath) -> Result<u8, String> {
    let report = check_keys(root)?;
    for problem in &report.problems {
        errln!("bench keys: {problem}");
    }
    if !report.problems.is_empty() {
        return Ok(EXIT_MISMATCH);
    }
    outln!(
        "bench keys: {} recomputed, {} source-derived keys not independently recomputed, {} agree with the recorded engine output",
        report.computed,
        report.not_recomputed,
        report.engine_checked
    );
    Ok(EXIT_OK)
}

fn run_score(root: &PyPath, path: &str, json: bool) -> Result<u8, String> {
    let keys = load_keys(root)?;
    let body = builder::read_text(&PyPath::new(path)).map_err(|e| format!("answers file: {e}"))?;
    let answers = pyjson::loads(&body).map_err(|e| format!("answers file: {e}"))?;
    let scored = score(&keys, &answers)?;
    let totals = tally(&scored);
    if json {
        let int = |n: usize| Json::Int(BigInt::from(n));
        let word = |m: String| Json::Str(m.to_lowercase());
        let payload = Json::Object(vec![
            (
                "totals".into(),
                Json::Object(
                    totals
                        .iter()
                        .map(|(k, v)| (k.to_string(), int(*v)))
                        .collect(),
                ),
            ),
            (
                "questions".into(),
                Json::Array(
                    scored
                        .iter()
                        .map(|s| {
                            Json::Object(vec![
                                ("id".into(), Json::str_obj(&s.id)),
                                ("value".into(), word(format!("{:?}", s.value))),
                                ("authority".into(), word(format!("{:?}", s.cited))),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]);
        outln!("{}", pyjson::dumps(&payload, false));
    } else {
        for (label, count) in &totals {
            outln!("{}: {count}", label.replace('_', " "));
        }
    }
    Ok(EXIT_OK)
}

pub fn main(root: Option<&str>, args: Args) -> u8 {
    let result = crate::root_for(root)
        .map_err(|e| e.to_string())
        .and_then(|root| match args.command {
            Command::Keys => run_keys(&root),
            Command::Score { answers, json } => run_score(&root, &answers, json),
        });
    result.unwrap_or_else(|e| {
        errln!("bench: {e}");
        EXIT_COULD_NOT_RUN
    })
}
