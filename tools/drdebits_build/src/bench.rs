// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The published benchmarks: questions, documented keys and a scorer, one
//! folder per topic under `benchmarks/`. Nothing here calls a model.
//!
//! `bench keys` recomputes each key that follows from a rule, from the inputs
//! the key records and the published tables its topic copies: a calendar of
//! non-business days, or the coefficients of the ATO's Schedule 1 withholding
//! formulas. It does not read the sources of the other keys. For the Payday
//! Super and Division 7A topic it also takes each engine value from the
//! recorded output and compares it with the key. `bench score` marks a file of
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
//! questions. See each topic's `README.md`.

use std::collections::HashSet;

use num_bigint::BigInt;

use crate::builder;
use crate::evals::json_int;
use crate::gate::{self, Authority, Q};
use crate::py::{Date, PyPath};
use crate::pyjson::{self, Json};
use crate::templates::BENCHMARK_CANARY;

/// A published topic: its folder under `benchmarks/`, what its `keys.json`
/// must say about several listed authorities (because the scorer applies
/// exactly this), and whether each key is also compared with a recorded
/// engine output in `engine-check.json`.
struct Topic {
    name: &'static str,
    authority_rule: &'static str,
    engine: bool,
}

const TOPICS: [Topic; 2] = [
    Topic {
        name: "payday-div7a",
        authority_rule: "Each section listed under a key is enough alone. For Payday Super keys either s 6 or s 18C counts as right, because each states the period relied on.",
        engine: true,
    },
    Topic {
        name: "payg-medicare",
        authority_rule: "Each section listed under a key is enough alone.",
        engine: false,
    },
];

/// The topic `bench` uses when none is named.
pub const DEFAULT_TOPIC: &str = "payday-div7a";

impl Topic {
    fn find(name: &str) -> Result<&'static Topic, String> {
        TOPICS
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| format!("unknown topic {name}"))
    }

    fn file(&self, name: &str) -> String {
        format!("benchmarks/{}/{name}", self.name)
    }
}

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
    /// Schedule 1's amount to withhold for a period, less any tax offsets
    /// claimed and any Medicare levy adjustment for `children` (0 for a
    /// spouse and no children).
    Withholding {
        scale: String,
        period: Period,
        earnings: Q,
        offsets: Option<Q>,
        children: Option<u32>,
    },
    /// Schedule 1's Medicare levy adjustment for a period.
    LevyAdjustment {
        scale: String,
        period: Period,
        earnings: Q,
        children: u32,
    },
    /// The Medicare levy under ss 6, 7 and 8 of the Medicare Levy Act 1986.
    MedicareLevy {
        taxable_income: Q,
        threshold_amount: Q,
        phase_in_limit: Q,
        family: Option<Family>,
    },
}

/// How often Schedule 1 earnings are paid.
#[derive(Clone, Copy, Debug)]
enum Period {
    Weekly,
    Fortnightly,
    Monthly,
}

impl Period {
    fn read(item: &Json, at: &str) -> Result<Period, String> {
        match text(item, "period", at)? {
            "weekly" => Ok(Period::Weekly),
            "fortnightly" => Ok(Period::Fortnightly),
            "monthly" => Ok(Period::Monthly),
            other => Err(format!("{at}: unknown period {other}")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Period::Weekly => "weekly",
            Period::Fortnightly => "fortnightly",
            Period::Monthly => "monthly",
        }
    }
}

/// A married person's s 8 facts: the family income threshold and the
/// spouse's taxable income and own threshold amount.
#[derive(Debug)]
struct Family {
    threshold: Q,
    spouse_income: Q,
    spouse_threshold: Q,
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
        "withholding" => Ok(Rule::Withholding {
            scale: text(check, "scale", at)?.to_string(),
            period: Period::read(check, at)?,
            earnings: decimal_field(check, "earnings", at)?,
            offsets: optional(check, "offsets", |_| decimal_field(check, "offsets", at))?,
            children: optional(check, "adjustment", |a| whole(a, "children", at))?,
        }),
        "levy-adjustment" => Ok(Rule::LevyAdjustment {
            scale: text(check, "scale", at)?.to_string(),
            period: Period::read(check, at)?,
            earnings: decimal_field(check, "earnings", at)?,
            children: whole(check, "children", at)?,
        }),
        "medicare-levy" => Ok(Rule::MedicareLevy {
            taxable_income: decimal_field(check, "taxable_income", at)?,
            threshold_amount: decimal_field(check, "threshold_amount", at)?,
            phase_in_limit: decimal_field(check, "phase_in_limit", at)?,
            family: optional(check, "spouse", |spouse| {
                Ok(Family {
                    threshold: decimal_field(check, "family_income_threshold", at)?,
                    spouse_income: decimal_field(spouse, "taxable_income", at)?,
                    spouse_threshold: decimal_field(spouse, "threshold_amount", at)?,
                })
            })?,
        }),
        other => Err(format!("{at}: unknown rule {other}")),
    }
}

/// `read` applied to `field` when the check has it.
fn optional<T>(
    check: &Json,
    field: &str,
    read: impl FnOnce(&Json) -> Result<T, String>,
) -> Result<Option<T>, String> {
    check.get(field).map(read).transpose()
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

/// Read a topic's keys, refusing a set that does not answer the questions one
/// for one.
pub fn load_keys(root: &PyPath, topic: &str) -> Result<Vec<Key>, String> {
    let topic = Topic::find(topic)?;
    let (questions_file, keys_file) = (topic.file("questions.json"), topic.file("keys.json"));
    let questions = with_canary(read_json(root, &questions_file)?, &questions_file)?;
    let keys = with_canary(read_json(root, &keys_file)?, &keys_file)?;
    if keys.get("authority_rule").and_then(Json::as_str) != Some(topic.authority_rule) {
        return Err(format!("{keys_file}: authority_rule is missing or changed"));
    }
    let asked = items(&questions, "questions", &questions_file)?
        .iter()
        .map(|q| text(q, "id", &questions_file))
        .collect::<Result<Vec<_>, _>>()?;
    let keys = items(&keys, "keys", &keys_file)?
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

fn load_calendar(root: &PyPath, file: &str) -> Result<Calendar, String> {
    let payload = read_json(root, file)?;
    let closed = items(&payload, "non_business_days", file)?
        .iter()
        .map(|d| date(text(d, "date", file)?, file))
        .collect::<Result<_, _>>()?;
    let bound = |field| date(text(&payload, field, file)?, file);
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

fn ratio(n: u32, d: u32) -> Q {
    Q::new(BigInt::from(n), BigInt::from(d))
}

/// `q` rounded to `places`, a half rounded up, as Schedule 1 and the levy
/// round amounts that are never negative.
fn rounded(q: &Q, places: u32) -> Option<Q> {
    amount(&gate::decimal(q, places))
}

/// Schedule 1's `x`: the period's earnings as weekly earnings in whole dollars
/// plus 99 cents. Monthly earnings ending in 33 cents gain a cent first.
fn schedule_x(period: Period, earnings: &Q) -> Q {
    let weekly = match period {
        Period::Weekly => earnings.clone(),
        Period::Fortnightly => earnings.mul(&ratio(1, 2)),
        Period::Monthly => {
            let cents = earnings.add(&earnings.floor(), -1).mul(&ratio(100, 1));
            let thirds = if cents.equals(&ratio(33, 1)) {
                earnings.add(&ratio(1, 100), 1)
            } else {
                earnings.clone()
            };
            thirds.mul(&ratio(3, 13))
        }
    };
    weekly.floor().add(&ratio(99, 100), 1)
}

/// A rounded weekly amount as the period's amount: doubled for a fortnight,
/// and for a month multiplied by 13, divided by 3 and rounded to the dollar.
fn per_period(period: Period, weekly: &Q) -> Option<Q> {
    match period {
        Period::Weekly => Some(weekly.clone()),
        Period::Fortnightly => Some(weekly.mul(&ratio(2, 1))),
        Period::Monthly => rounded(&weekly.mul(&ratio(13, 3)), 0),
    }
}

fn table<'a>(schedule: &'a Json, field: &str, entry: &str, at: &str) -> Result<&'a Json, String> {
    schedule
        .get(field)
        .and_then(|t| t.get(entry))
        .ok_or_else(|| format!("{at}: Schedule 1 has no {field} for {entry}"))
}

/// `y = ax − b` from the first row of the scale that `x` is below, rounded to
/// the dollar. A row with no coefficients withholds nothing.
fn weekly_withholding(schedule: &Json, scale: &str, x: &Q, at: &str) -> Result<Q, String> {
    let Json::Array(rows) = table(schedule, "scales", scale, at)? else {
        return Err(format!("{at}: scale {scale} must be a list"));
    };
    for row in rows {
        let below = match row.get("below") {
            Some(Json::Null) => None,
            _ => Some(decimal_field(row, "below", at)?),
        };
        if below.is_some_and(|b| !x.less_than(&b)) {
            continue;
        }
        if matches!(row.get("a"), Some(Json::Null)) {
            return Ok(ratio(0, 1));
        }
        let (a, b) = (decimal_field(row, "a", at)?, decimal_field(row, "b", at)?);
        return rounded(&a.mul(x).add(&b, -1), 0)
            .filter(|y| !y.less_than(&ratio(0, 1)))
            .ok_or(format!("{at}: y is negative"));
    }
    Err(format!(
        "{at}: no row of scale {scale} covers {}",
        gate::decimal(x, 2)
    ))
}

/// Schedule 1's weekly Medicare levy adjustment (WLA) for `x`, from the
/// scale's weekly family threshold (WFT) and shading out point (SOP).
fn weekly_levy_adjustment(
    schedule: &Json,
    scale: &str,
    x: &Q,
    children: u32,
    at: &str,
) -> Result<Q, String> {
    let parameters = table(schedule, "medicare_levy_parameters", scale, at)?;
    let get = |field| decimal_field(parameters, field, at);
    if x.less_than(&get("weekly_earnings_threshold")?) {
        return Ok(ratio(0, 1));
    }
    let family =
        get("family_threshold")?.add(&get("additional_child")?.mul(&ratio(children, 1)), 1);
    let fail = || format!("{at}: the levy adjustment cannot be worked");
    let wft = family
        .div(&get("weekly_family_threshold_divisor")?)
        .and_then(|w| rounded(&w, 2))
        .ok_or_else(fail)?;
    let (multiplier, divisor, levy) = (
        get("shading_out_point_multiplier")?,
        get("shading_out_point_divisor")?,
        get("medicare_levy")?,
    );
    let sop = wft.mul(&multiplier).div(&divisor).ok_or_else(fail)?.floor();
    let wla = if !x.less_than(&sop) {
        ratio(0, 1)
    } else if x.less_than(&get("weekly_earnings_shade_in_threshold")?) {
        x.add(&get("weekly_levy_adjustment_factor")?, -1)
            .mul(&multiplier)
    } else if x.less_than(&wft) {
        x.mul(&levy)
    } else {
        wft.mul(&levy).add(&x.add(&wft, -1).mul(&divisor), -1)
    };
    rounded(&wla, 0).ok_or_else(fail)
}

/// The levy on `income`: none up to the threshold amount (s 7(1)), at most 10%
/// of the excess up to the phase-in limit (s 7(2)), otherwise 2% (s 6(1)),
/// then reduced under s 8(1) and (2) for a married person whose spouse would
/// not be liable. A liable spouse brings in ss 8(3) and 8(4), which share the
/// reduction and pass any excess between the spouses using the spouse's own
/// levy and family income threshold; the key does not record those, so that
/// case is refused rather than worked in part.
fn medicare_levy(
    income: &Q,
    threshold: &Q,
    phase_in: &Q,
    family: Option<&Family>,
) -> Result<Q, &'static str> {
    let two_percent = ratio(2, 100);
    let full = income.mul(&two_percent);
    let shaded = income.add(threshold, -1).mul(&ratio(1, 10));
    let levy = if !threshold.less_than(income) {
        ratio(0, 1)
    } else if !phase_in.less_than(income) && shaded.less_than(&full) {
        shaded
    } else {
        full
    };
    let Some(family) = family else {
        return Ok(levy);
    };
    let family_income = income.add(&family.spouse_income, 1);
    if !family.threshold.less_than(&family_income) {
        return Ok(ratio(0, 1));
    }
    if family.spouse_threshold.less_than(&family.spouse_income) {
        return Err(
            "the spouse would be liable, so ss 8(3) and 8(4) apply, which this rule does not work",
        );
    }
    let excess = family_income.add(&family.threshold, -1);
    let reduction = family
        .threshold
        .mul(&two_percent)
        .add(&excess.mul(&ratio(8, 100)), -1);
    Ok(if !ratio(0, 1).less_than(&reduction) {
        levy
    } else if reduction.less_than(&levy) {
        levy.add(&reduction, -1)
    } else {
        ratio(0, 1)
    })
}

/// The published tables a topic's rules read, loaded only when a rule needs one.
struct Tables {
    calendar: Option<Calendar>,
    schedule: Option<Json>,
}

/// The value a key's rule gives, or `None` for a key not recomputed here.
fn reproduce(key: &Key, tables: &Tables) -> Result<Option<String>, String> {
    let fail = || format!("{}: the rule cannot be worked", key.id);
    let schedule = || tables.schedule.as_ref().ok_or_else(fail);
    Ok(match &key.rule {
        Rule::Source => None,
        Rule::BusinessDays(terms) => {
            let calendar = tables.calendar.as_ref().ok_or_else(fail)?;
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
        Rule::Withholding {
            scale,
            period,
            earnings,
            offsets,
            children,
        } => {
            let (schedule, x) = (schedule()?, schedule_x(*period, earnings));
            let gross = weekly_withholding(schedule, scale, &x, &key.id)?;
            let mut net = per_period(*period, &gross).ok_or_else(fail)?;
            if let Some(offsets) = offsets {
                if !["2", "5", "6"].contains(&scale.as_str()) {
                    return Err(format!(
                        "{}: Schedule 1 allows tax offsets only on scales 2, 5 and 6",
                        key.id
                    ));
                }
                let rate = table(schedule, "offset_rates", period.name(), &key.id)?;
                let rate = rate.as_str().and_then(amount).ok_or_else(fail)?;
                net = net.add(&rounded(&offsets.mul(&rate), 0).ok_or_else(fail)?, -1);
            }
            if let Some(children) = children {
                let wla = weekly_levy_adjustment(schedule, scale, &x, *children, &key.id)?;
                net = net.add(&per_period(*period, &wla).ok_or_else(fail)?, -1);
            }
            if net.less_than(&ratio(0, 1)) {
                return Err(fail());
            }
            Some(gate::decimal(&net, 2))
        }
        Rule::LevyAdjustment {
            scale,
            period,
            earnings,
            children,
        } => {
            let x = schedule_x(*period, earnings);
            let wla = weekly_levy_adjustment(schedule()?, scale, &x, *children, &key.id)?;
            Some(gate::decimal(
                &per_period(*period, &wla).ok_or_else(fail)?,
                2,
            ))
        }
        Rule::MedicareLevy {
            taxable_income,
            threshold_amount,
            phase_in_limit,
            family,
        } => {
            let levy = medicare_levy(
                taxable_income,
                threshold_amount,
                phase_in_limit,
                family.as_ref(),
            )
            .map_err(|why| format!("{}: {why}", key.id))?;
            Some(gate::decimal(&levy, 2))
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

/// Recompute what can be recomputed and, for a topic with engine records,
/// compare every key with the value taken from the recorded engine output. A
/// duplicate record stops the check.
pub fn check_keys(root: &PyPath, topic: &str) -> Result<KeyReport, String> {
    let keys = load_keys(root, topic)?;
    let topic = Topic::find(topic)?;
    let needs = |rule: fn(&Rule) -> bool| keys.iter().any(|k| rule(&k.rule));
    let tables = Tables {
        calendar: needs(|r| matches!(r, Rule::BusinessDays(_)))
            .then(|| load_calendar(root, &topic.file("business-days.json")))
            .transpose()?,
        schedule: needs(|r| matches!(r, Rule::Withholding { .. } | Rule::LevyAdjustment { .. }))
            .then(|| read_json(root, &topic.file("schedule-1.json")))
            .transpose()?,
    };
    let engine_file = topic.file("engine-check.json");
    let engine = topic
        .engine
        .then(|| read_json(root, &engine_file))
        .transpose()?;
    let checks = match &engine {
        Some(engine) => items(engine, "checks", &engine_file)?,
        None => &[],
    };
    let mut ids = HashSet::new();
    for check in checks {
        let id = text(check, "id", &engine_file)?;
        if !ids.insert(id) {
            return Err(format!("{engine_file}: {id} is recorded twice"));
        }
    }
    let mut report = KeyReport {
        computed: 0,
        not_recomputed: 0,
        engine_checked: 0,
        problems: Vec::new(),
    };
    for key in &keys {
        match reproduce(key, &tables)? {
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
        if !topic.engine {
            continue;
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
        let at = format!("{engine_file} {}", key.id);
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

/// What `bench compare` counts for each answer, with the scorer's meanings:
/// authority is right only on an answered question, as in `tally`.
type Measure = (&'static str, fn(&Scored) -> bool);
const MEASURES: [Measure; 5] = [
    ("value correct", |s| s.value == Value::Correct),
    ("value wrong", |s| s.value == Value::Wrong),
    ("unanswered", |s| s.value == Value::Unanswered),
    ("authority right", |s| {
        s.value != Value::Unanswered && s.cited == Cited::Right
    }),
    ("correct value and authority", |s| {
        s.value == Value::Correct && s.cited == Cited::Right
    }),
];

/// One measure compared between conditions a and b. Each question is one
/// pair: the number of repeats in which a's answer has the measure against
/// the number in which b's does.
#[derive(Debug, PartialEq)]
pub struct Paired {
    pub measure: &'static str,
    pub a_total: usize,
    pub b_total: usize,
    pub a_higher: usize,
    pub b_higher: usize,
    pub ties: usize,
    /// The exact two-sided sign test on the untied questions, to 4 places:
    /// `p = 0.1250`, or `p < 0.0001` when that would show as 0.
    pub p: String,
}

/// The fewest untied questions whose most uneven split gives p below 0.05:
/// 5 allow 2 / 2^5 = 0.0625 at best, 6 allow 2 / 2^6 = 0.03125.
const UNTIED_FOR_P_BELOW_0_05: usize = 6;

/// The exact two-sided sign test for `a` pairs one way and `b` the other,
/// ties left out: twice the chance that `a + b` fair coin tosses split at
/// least as unevenly, and never more than 1. With no untied pair it is 1.
pub fn sign_test(a: usize, b: usize) -> String {
    let n = a + b;
    let choose = |k: usize| {
        (0..k).fold(BigInt::from(1), |c, i| {
            c * BigInt::from(n - i) / BigInt::from(i + 1)
        })
    };
    let twice_tail: BigInt = (0..=a.min(b)).map(choose).sum::<BigInt>() * 2;
    let outcomes = BigInt::from(1) << n;
    let p = Q::new(twice_tail.clone().min(outcomes.clone()), outcomes);
    match gate::decimal(&p, 4).as_str() {
        "0.0000" => "p < 0.0001".into(),
        shown => format!("p = {shown}"),
    }
}

/// Score every repeat of 2 conditions and pair them question by question.
/// Refuses a condition with no answers file, conditions with different
/// numbers of repeats, which cannot be paired, and any file the scorer
/// refuses, naming its condition and place.
pub fn compare(keys: &[Key], a: &[Json], b: &[Json]) -> Result<Vec<Paired>, String> {
    if a.is_empty() || b.is_empty() {
        return Err("each condition needs at least one answers file".into());
    }
    if a.len() != b.len() {
        return Err(format!(
            "the conditions are unpaired: a has {} repeats and b has {}",
            a.len(),
            b.len()
        ));
    }
    let scored = |side: &str, files: &[Json]| {
        files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                score(keys, f).map_err(|e| format!("{e} (condition {side}, file {})", i + 1))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let (a, b) = (scored("a", a)?, scored("b", b)?);
    Ok(MEASURES
        .iter()
        .map(|&(measure, has)| {
            let count = |runs: &[Vec<Scored>], q: usize| runs.iter().filter(|r| has(&r[q])).count();
            let pairs: Vec<(usize, usize)> = (0..keys.len())
                .map(|q| (count(&a, q), count(&b, q)))
                .collect();
            let a_higher = pairs.iter().filter(|(x, y)| x > y).count();
            let b_higher = pairs.iter().filter(|(x, y)| x < y).count();
            Paired {
                measure,
                a_total: pairs.iter().map(|p| p.0).sum(),
                b_total: pairs.iter().map(|p| p.1).sum(),
                a_higher,
                b_higher,
                ties: pairs.len() - a_higher - b_higher,
                p: sign_test(a_higher, b_higher),
            }
        })
        .collect())
}

#[derive(clap::Args)]
pub struct Args {
    /// The benchmark folder under `benchmarks/`: payday-div7a or payg-medicare.
    #[arg(long, global = true, default_value = DEFAULT_TOPIC)]
    topic: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Recompute the keys and compare them with any recorded engine output.
    Keys,
    /// Mark a JSON answers file on value and authority.
    Score {
        /// The answers file.
        answers: String,
        /// Print each question's marks and the totals as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Compare 2 conditions question by question across equal repeats, with
    /// an exact two-sided sign test on each measure.
    Compare {
        /// Condition a's answers files, one per repeat.
        #[arg(long = "a", num_args = 1.., required = true)]
        a: Vec<String>,
        /// Condition b's answers files, one per repeat, as many as a's.
        #[arg(long = "b", num_args = 1.., required = true)]
        b: Vec<String>,
    },
}

fn run_keys(root: &PyPath, topic: &str) -> Result<u8, String> {
    let report = check_keys(root, topic)?;
    for problem in &report.problems {
        errln!("bench keys: {problem}");
    }
    if !report.problems.is_empty() {
        return Ok(EXIT_MISMATCH);
    }
    let engine = if Topic::find(topic)?.engine {
        format!(
            ", {} agree with the recorded engine output",
            report.engine_checked
        )
    } else {
        String::new()
    };
    outln!(
        "bench keys: {} recomputed, {} source-derived keys not independently recomputed{engine}",
        report.computed,
        report.not_recomputed,
    );
    Ok(EXIT_OK)
}

fn read_answers(path: &str) -> Result<Json, String> {
    let body = builder::read_text(&PyPath::new(path)).map_err(|e| format!("answers file: {e}"))?;
    pyjson::loads(&body).map_err(|e| format!("answers file: {e}"))
}

fn run_score(root: &PyPath, topic: &str, path: &str, json: bool) -> Result<u8, String> {
    let keys = load_keys(root, topic)?;
    let scored = score(&keys, &read_answers(path)?)?;
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

fn run_compare(root: &PyPath, topic: &str, a: &[String], b: &[String]) -> Result<u8, String> {
    let keys = load_keys(root, topic)?;
    // The same file under 2 spellings, such as `x.json` and `./x.json`, is
    // still one file; a path that does not resolve fails when it is read.
    let mut seen = HashSet::new();
    let file = |p: &String| std::fs::canonicalize(p).unwrap_or_else(|_| p.into());
    if let Some(twice) = a.iter().chain(b).find(|p| !seen.insert(file(p))) {
        return Err(format!(
            "{twice} is given more than once: each answers file is one repeat of one condition"
        ));
    }
    let read = |side: &str, paths: &[String]| {
        paths
            .iter()
            .map(|p| read_answers(p).map_err(|e| format!("{e} (condition {side}, {p})")))
            .collect::<Result<Vec<_>, _>>()
    };
    let paired = compare(&keys, &read("a", a)?, &read("b", b)?)?;
    let repeats = a.len();
    let mean = |total: usize| gate::decimal(&Q::new(BigInt::from(total), BigInt::from(repeats)), 2);
    outln!(
        "bench compare: {} questions paired by question; answers files per condition: {repeats}",
        keys.len()
    );
    for m in &paired {
        let few = if m.a_higher + m.b_higher < UNTIED_FOR_P_BELOW_0_05 {
            "; too few untied questions for any split to give p below 0.05"
        } else {
            ""
        };
        outln!(
            "{}: a {} (mean {}), b {} (mean {}); a higher on {}, b higher on {}, ties {}; two-sided sign test {}{few}",
            m.measure,
            m.a_total,
            mean(m.a_total),
            m.b_total,
            mean(m.b_total),
            m.a_higher,
            m.b_higher,
            m.ties,
            m.p
        );
    }
    Ok(EXIT_OK)
}

pub fn main(root: Option<&str>, args: Args) -> u8 {
    let result = crate::root_for(root)
        .map_err(|e| e.to_string())
        .and_then(|root| match args.command {
            Command::Keys => run_keys(&root, &args.topic),
            Command::Score { answers, json } => run_score(&root, &args.topic, &answers, json),
            Command::Compare { a, b } => run_compare(&root, &args.topic, &a, &b),
        });
    result.unwrap_or_else(|e| {
        errln!("bench: {e}");
        EXIT_COULD_NOT_RUN
    })
}
