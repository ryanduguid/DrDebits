// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: LicenseRef-Duguid-Public-Attribution-1.0

//! A deterministic gate a host runs on a high-impact draft before showing or
//! acting on it.
//!
//! The guide tells the model what a high-impact draft must contain; prompt
//! wording cannot make it comply. `gate` checks the parts a machine can check
//! and answers in four states:
//!
//! ```text
//! 0   PASS    every mechanical check passed; a human still reviews the draft
//! 1   BLOCK   the draft breaks the output contract or its arithmetic
//! 2   the gate could not run (unreadable response, guide sources or `--source` file)
//! 3   REVIEW  nothing blocked, but the prohibited-conclusion screen fired,
//!             the draft records a state-changing action or the calculation
//!             scan gave up on a line; a human decides
//! ```
//!
//! Checked: the draft banner; one primary decision status naming one allowed
//! high-impact status; `action_authority: NONE`; a `state_changing_tool_action`
//! line; both source-version status lines with an allowed value; the sections
//! the guide makes mandatory, each with content; and every written calculation
//! of the form `a op b = c`, including each link of a chain, reperformed in
//! exact decimal arithmetic. A minus sign needs spaces around it, so section
//! numbers such as `8-1` and years such as `2026-27` are not subtractions. A
//! lone bracketed amount such as `(120)` blocks as ambiguous, because it may
//! be an accounting negative. A bracket opened before a calculation, as in
//! `(2 + 3 = 5)`, belongs to the sentence. Any other bracket that does not
//! pair within its step between `=` signs blocks the calculation, even a
//! correct one: the gate sees only part of the sentence and cannot tell what
//! the bracket encloses. For the same reason a calculation blocks, even a
//! correct one, when an amount, an operator and words come before it on its
//! line: a sum that a word interrupts, as in
//! `$100 + wages $200 + $50 = $250`, and also a sum after such a label, as
//! in `Item 3 - Deductions $100 + $50 = $150`.
//!
//! Mandatory sections follow Risk classification for `PROCEED_DRAFT_ONLY` and
//! `NEEDS_FACTS`. An `ESCALATE` or `HARD_STOP` draft may be the abbreviated
//! record the Output contract allows, so it must keep the banner, status,
//! action fields and external-action record, and the other sections are
//! reported as notes. Sections the contract requires only "where material"
//! are reported as notes, because materiality is a judgement.
//!
//! Not checked: whether the facts, sources, analysis or conclusion are
//! right, whether a source is current, whether a refusal was required, or
//! whether the free-text scope, reason and next human step are adequate.
//!
//! With `--facts FILE`, each dollar amount (`$1,100.50`) and percentage
//! (`12%`) in the draft must be a supplied fact or the written result of a
//! calculation that reperforms and whose dollar and percentage inputs are
//! facts or results of earlier calculations; a running total in a chain counts
//! as a result. A result written without `$` is dollars when its expression
//! leaves no doubt: dollars plus or minus dollars, or dollars times or divided
//! by a plain number or percentage; dollars divided by dollars is a ratio.
//! Anything else asks for REVIEW, and so does any amount-like text the gate
//! cannot read whole, such as `$1.2 million`, `$5k`, `$ 1,200`, `$1 200`,
//! `.5%` or `10 %`; it is never matched by a readable prefix. This check
//! never blocks. Values compare exactly by magnitude: a sign, `$`, commas and
//! trailing zeros do not matter, a percentage matches only a percentage, and
//! a rounded mention of a fact does not match it. Plain numbers such as `11`
//! or `12` months are not amounts and are not checked, so a calculation from
//! plain numbers alone, such as `30 × 40 = $1,200`, supports its result.
//! Other currencies' symbols are not read. The check flags amounts it cannot
//! trace; an amount it traces is not thereby correct. For each amount equal
//! to a supplied fact, a note names that fact: a match in value, not evidence
//! that the figure came from it. An amount equal to 2 or more facts is noted
//! as ambiguous with every name, and the gate does not pick one.
//!
//! With `--authorities FILE`, each authority the draft cites in one of 2
//! forms must be in the file's closed list: a section of an Act named in
//! full, `s 8-1 of the Income Tax Assessment Act 1997` (`section`, `sec`,
//! `s.` or `§` may replace `s`, and `s8-1` needs no space), or one
//! subsection, paragraph or subparagraph of it with its bracketed part
//! (`subsection 8-1(1)`, `subs`, `sub-s`, `subsec`, `sub-sec`,
//! `paragraph 8-1(1)(a)`, `para`, `subparagraph`, `subpara`), and a
//! ruling id in one of the series `SERIES` lists, such as `TR 2006/10`,
//! `LCR 2026/D1` or `TPB(GS) 55/2026`. A composite citation such as
//! `paragraph (a) of subsection 9-5(1) of the ...` is read as the numbered
//! provision that ends it.
//! A citation may wrap across lines. A citation is listed when the list holds
//! it or a provision containing it, whitespace and line breaks aside: `s 8-1`
//! lists `subsection 8-1(1)` and `s 8-1(1)` lists `paragraph 8-1(1)(a)`, but
//! `s 8-1(1)` lists neither `s 8-1` nor `s 8-1(2)`, and the withdrawal
//! `TR 2010/3W` is not `TR 2010/3`. A citation not in the
//! list asks for REVIEW, and so does section-like or ruling-like text the gate
//! cannot read whole, such as `s 8-1 ITAA 1997`, `ss 30-35 and 30-40`,
//! `subsections 8-1(1) and (2)`, sibling parts after the Act such as
//! `, (2) and (3)`, a part named without its section, such as
//! `subsection (2)`, or `TR 2024`. A part named without its section asks for
//! REVIEW even in discussion after a full citation, as in
//! `subsection (1) allows`: the gate cannot tell which section it belongs to.
//! A
//! possessive such as `the client's 2 trusts` is not a section. Divisions,
//! Parts, Schedules, cases, bare Act names and other series are not read, so
//! a PASS does not show that every authority the draft cites was checked.
//! This check never blocks, and a listed authority is not thereby current or
//! correctly applied.
//!
//! With `--source FILE`, repeated for each source, each quoted span in the
//! draft must appear verbatim in a supplied source. A span is text of 2 or
//! more words in double quotes or single quotes, straight or curly, or a
//! Markdown blockquote; a single quoted word is a term, not a quotation, and
//! is not read. Punctuation ending a span is tried without it. A span that
//! matches only after whitespace or typography is normalised, a span not
//! found and a quotation whose end cannot be found each ask for REVIEW, and
//! so does a quotation shortened with an ellipsis, because the gate looks for
//! it whole. This check never blocks. A span found is text membership only: it
//! does not show that the source applies or supports the claim.

use std::sync::LazyLock;

use num_bigint::BigInt;
use num_traits::{Signed, Zero};

use crate::builder;
use crate::error::Fail;
use crate::harness;
use crate::model::Conclusion;
use crate::py::{self, PyPath};
use crate::pyjson::{self, Json};
use crate::quotecheck;

pub const EXIT_PASS: u8 = 0;
pub const EXIT_BLOCK: u8 = 1;
pub const EXIT_COULD_NOT_RUN: u8 = 2;
pub const EXIT_REVIEW: u8 = 3;

const FULL_STATUSES: [&str; 2] = ["PROCEED_DRAFT_ONLY", "NEEDS_FACTS"];
const STOP_STATUSES: [&str; 2] = ["ESCALATE", "HARD_STOP"];
/// Risk classification: mandatory for every high-impact task.
const MANDATORY: [&str; 5] = [
    "Scope and period",
    "Source-version status",
    "Assumptions",
    "Human actions before use",
    "External-action record",
];
/// Output contract fields required "where material".
const WHERE_MATERIAL: [&str; 5] = [
    "Facts and evidence",
    "Applicable authority",
    "Analysis",
    "Ethical and regulatory flags",
    "Conclusion and uncertainty",
];
const SOURCE_VALUES: [&str; 2] = ["CONFIRMED", "SOURCE CURRENCY NOT CONFIRMED"];

static BANNER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^DRAFT\s*[—–:-]\s*PROFESSIONAL REVIEW REQUIRED$").expect("pattern")
});
static FIELD: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^(action_authority|state_changing_tool_action|historical authority|current duties)\s*:\s*(.*)$")
        .expect("pattern")
});

const NUM: &str = r"\$?\d{1,3}(?:,\d{3})+(?:\.\d+)?%?|\$?\d+(?:\.\d+)?%?";
static CLAIM: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    // One line only: a bullet's leading hyphen is not a minus sign. Group 1
    // holds every link before the final result, each ending in `=`; the first
    // link needs an operator, so a label such as `Item 2 =` is not one.
    let operand = format!(r"\(*[ \t]*[-−]?(?:{NUM})[ \t]*\)*");
    let op = r"(?:[ \t]*[+×*/÷][ \t]*|[ \t]+[-−x][ \t]+)";
    fancy_regex::Regex::new(&format!(
        r"(?<![\w.,$/])({operand}(?:{op}{operand})+[ \t]*=[ \t]*(?:{operand}(?:{op}{operand})*[ \t]*=[ \t]*)*)(\([ \t]*[-−]?(?:{NUM})[ \t]*\)|[-−]?(?:{NUM}))(?![\d,.]*\d)(?![ \t]*[+×*/÷=])(?![ \t]+[-−x][ \t])"
    ))
    .expect("pattern")
});
static LEADING: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(&format!("^[-−]?(?:{NUM})")).expect("pattern"));
static NUMBER: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(NUM).expect("pattern"));
/// A lone unsigned bracketed amount, `(120)`: grouping, or an accounting
/// negative. `(-120)` carries its sign and is not ambiguous.
static BRACKETED: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(&format!(r"\([ \t]*(?:{NUM})[ \t]*\)")).expect("pattern"));
/// The text before a calculation ends in an amount, an operator and words,
/// which may hold digits, operators and brackets, as `$100 + wages `,
/// `$100 + (wages) + ` and `$100 + FY2025 wages ` do before
/// `$200 + $50 = $250`.
static INTERRUPTED: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(
        r"(?:{NUM})(?:[ \t]*[+×*/÷][ \t]*|[ \t]+[-−x][ \t]+)[^=$]*\p{{L}}[^=$]*$"
    ))
    .expect("pattern")
});
static DATE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\d{1,2}/\d{1,2}/\d{2,4}").expect("pattern"));
static NUMBERED: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\d{1,2}[.)]?\s+").expect("pattern"));

/// An exact rational: `n / d` with `d > 0`.
#[derive(Clone, Debug)]
pub(crate) struct Q {
    n: BigInt,
    d: BigInt,
}

impl Q {
    pub(crate) fn new(n: BigInt, d: BigInt) -> Q {
        if d.is_negative() {
            Q { n: -n, d: -d }
        } else {
            Q { n, d }
        }
    }
    pub(crate) fn add(&self, o: &Q, sign: i32) -> Q {
        Q::new(
            &self.n * &o.d + BigInt::from(sign) * &o.n * &self.d,
            &self.d * &o.d,
        )
    }
    pub(crate) fn mul(&self, o: &Q) -> Q {
        Q::new(&self.n * &o.n, &self.d * &o.d)
    }
    pub(crate) fn div(&self, o: &Q) -> Option<Q> {
        (!o.n.is_zero()).then(|| Q::new(&self.n * &o.d, &self.d * &o.n))
    }
    fn neg(&self) -> Q {
        Q::new(-&self.n, self.d.clone())
    }
    pub(crate) fn equals(&self, o: &Q) -> bool {
        &self.n * &o.d == &o.n * &self.d
    }
}

/// A number as written: `$1,100.50`, `10%`, `−3`. Returns its value and the
/// count of decimal places it shows (a percentage counts in percent).
pub(crate) fn number(text: &str) -> Option<(Q, u32, bool)> {
    let negative = text.starts_with(['-', '−']);
    let body = text.trim_start_matches(['-', '−']).trim_start_matches('$');
    let percent = body.ends_with('%');
    let digits = body.trim_end_matches('%').replace(',', "");
    let (whole, frac) = digits.split_once('.').unwrap_or((&digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let places = u32::try_from(frac.len()).ok()?;
    let mut n: BigInt = format!("{whole}{frac}").parse().ok()?;
    let mut d = BigInt::from(10).pow(places);
    if percent {
        d *= 100;
    }
    if negative {
        n = -n;
    }
    Some((Q::new(n, d), places, percent))
}

/// What an expression's value measures, so a result written without `$`
/// can still be dollars. A percentage is a plain number here.
#[derive(Clone, Copy, PartialEq)]
enum Unit {
    Dollars,
    Plain,
    Unknown,
}

impl Unit {
    /// Dollars only when the operation leaves no doubt: dollars plus or minus
    /// dollars, dollars times a plain number, or dollars divided by one.
    /// Dollars divided by dollars is a plain ratio.
    fn combine(op: char, a: Unit, b: Unit) -> Unit {
        use Unit::{Dollars, Plain, Unknown};
        match (op, a, b) {
            (_, Plain, Plain) | ('/', Dollars, Dollars) => Plain,
            ('+' | '-', Dollars, Dollars)
            | ('*', Dollars, Plain)
            | ('*', Plain, Dollars)
            | ('/', Dollars, Plain) => Dollars,
            _ => Unknown,
        }
    }
}

enum Token {
    Num(Q, Unit),
    Op(char),
}

fn tokens(text: &str) -> Option<Vec<Token>> {
    let number_at = regex::Regex::new(&format!("^(?:{NUM})")).expect("pattern");
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c.is_whitespace() {
            rest = &rest[c.len_utf8()..];
        } else if let Some(m) = number_at.find(rest) {
            let unit = if m.as_str().starts_with('$') {
                Unit::Dollars
            } else {
                Unit::Plain
            };
            out.push(Token::Num(number(m.as_str())?.0, unit));
            rest = &rest[m.end()..];
        } else {
            let op = match c {
                '+' | '(' | ')' | '/' => c,
                '-' | '−' => '-',
                '×' | '*' | 'x' => '*',
                '÷' => '/',
                _ => return None,
            };
            out.push(Token::Op(op));
            rest = &rest[c.len_utf8()..];
        }
    }
    Some(out)
}

/// Standard precedence: unary minus, then `*` and `/`, then `+` and `-`.
struct Parser {
    tokens: Vec<Token>,
    at: usize,
    divided_by_zero: bool,
}

impl Parser {
    fn op(&self) -> Option<char> {
        match self.tokens.get(self.at) {
            Some(Token::Op(c)) => Some(*c),
            _ => None,
        }
    }
    fn expr(&mut self) -> Option<(Q, Unit)> {
        let (mut value, mut unit) = self.term()?;
        while let Some(op @ ('+' | '-')) = self.op() {
            self.at += 1;
            let (right, right_unit) = self.term()?;
            value = value.add(&right, if op == '+' { 1 } else { -1 });
            unit = Unit::combine(op, unit, right_unit);
        }
        Some((value, unit))
    }
    fn term(&mut self) -> Option<(Q, Unit)> {
        let (mut value, mut unit) = self.factor()?;
        while let Some(op @ ('*' | '/')) = self.op() {
            self.at += 1;
            let (right, right_unit) = self.factor()?;
            value = if op == '*' {
                value.mul(&right)
            } else {
                let quotient = value.div(&right);
                self.divided_by_zero |= quotient.is_none();
                quotient?
            };
            unit = Unit::combine(op, unit, right_unit);
        }
        Some((value, unit))
    }
    fn factor(&mut self) -> Option<(Q, Unit)> {
        match self.tokens.get(self.at) {
            Some(Token::Op('-')) => {
                self.at += 1;
                let (value, unit) = self.factor()?;
                Some((value.neg(), unit))
            }
            Some(Token::Op('(')) => {
                self.at += 1;
                let value = self.expr()?;
                (self.op() == Some(')')).then(|| self.at += 1)?;
                Some(value)
            }
            Some(Token::Num(q, unit)) => {
                let found = (q.clone(), *unit);
                self.at += 1;
                Some(found)
            }
            _ => None,
        }
    }
}

/// The value and unit of an expression: `None` when it does not parse,
/// `Some(None)` when it divides by zero. `first` replaces the unit of the
/// first number, a running total carried from the previous link.
fn parse(expression: &str, first: Option<Unit>) -> Option<Option<(Q, Unit)>> {
    let mut tokens = tokens(expression)?;
    if let (Some(carried), Some(Token::Num(_, unit))) = (
        first,
        tokens.iter_mut().find(|t| matches!(t, Token::Num(..))),
    ) {
        *unit = carried;
    }
    let mut parser = Parser {
        tokens,
        at: 0,
        divided_by_zero: false,
    };
    let value = parser.expr();
    if parser.divided_by_zero {
        return Some(None);
    }
    (parser.at == parser.tokens.len()).then_some(Some(value?))
}

/// The value of an expression, as `parse` gives it.
fn evaluate(expression: &str) -> Option<Option<Q>> {
    parse(expression, None).map(|v| v.map(|(q, _)| q))
}

/// Whether `a` is within one unit of the last of `places` decimal places of
/// `b`, counting in percent when `percent` is set; compared as integers.
fn close(a: &Q, b: &Q, places: u32, percent: bool) -> bool {
    let scale = BigInt::from(10).pow(places) * if percent { 100 } else { 1 };
    let difference = (&a.n * &b.d - &b.n * &a.d).abs();
    difference * scale < &a.d * &b.d
}

pub(crate) fn decimal(q: &Q, places: u32) -> String {
    // Half away from zero, for display only: floor(|n| * 10^places / d + 1/2).
    let scaled: BigInt = q.n.abs() * BigInt::from(10).pow(places);
    let magnitude: BigInt = (scaled * 2 + &q.d) / (&q.d * 2);
    let digits = magnitude.to_string();
    let places = places as usize;
    let padded = format!("{digits:0>width$}", width = places + 1);
    let (whole, frac) = padded.split_at(padded.len() - places);
    let sign = if q.n.is_negative() && !magnitude.is_zero() {
        "-"
    } else {
        ""
    };
    if places == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{frac}")
    }
}

/// One written calculation and whether it reperforms.
pub struct Calculation {
    pub text: String,
    pub computed: String,
    pub matches: bool,
    /// Numbers as written that the calculation takes as inputs.
    pub operands: Vec<String>,
    /// Numbers that the calculation produces: its final result and each
    /// running total a later link starts from, as written, except that a
    /// result written without `$` gains one when the expression it ends is
    /// unambiguously in dollars.
    pub results: Vec<String>,
}

/// Every `a op b = c` in the text, reperformed, and the number of each line
/// the scan gave up on. The claimed result matches
/// when it is within one unit of its last decimal place, so a result rounded
/// up, down or to nearest passes and a wrong digit does not. In a chain
/// `a op b = c op d = e`, each link must equal either the whole next
/// expression or its leading number, as a running total writes it.
pub fn calculations(text: &str) -> (Vec<Calculation>, Vec<usize>) {
    let mut out = Vec::new();
    let mut unscanned = Vec::new();
    let lines: Vec<String> = py::splitlines(text).into_iter().map(normalise).collect();
    let scanned = lines.iter().enumerate().flat_map(|(index, line)| {
        CLAIM
            .captures_iter(line.as_str())
            .map(move |m| (index + 1, line.as_str(), m))
    });
    for (line_number, line, m) in scanned {
        // An error is the pattern's backtracking limit; the line's iterator
        // ends after it.
        let Ok(m) = m else {
            unscanned.push(line_number);
            continue;
        };
        let (whole, left, right) = (&m[0], &m[1], &m[2]);
        if DATE.is_match(left) {
            continue;
        }
        let text = py::strip(whole).to_string();
        if BRACKETED.is_match(whole) {
            out.push(Calculation {
                text,
                computed: "ambiguous: a bracketed amount may be negative; write a minus sign"
                    .into(),
                matches: false,
                operands: Vec::new(),
                results: Vec::new(),
            });
            continue;
        }
        // The tail of a sum that a word interrupts, as in `$200 + $50 = $250`
        // after `$100 + wages `: the gate cannot tell what the sum adds.
        let start = m.get(0).expect("whole match").start();
        if INTERRUPTED.is_match(&line[..start]) {
            out.push(Calculation {
                text,
                computed: "could not read: a word interrupts the sum before this step, so the gate sees only its tail; write the calculation on a line of its own, without words between its amounts".into(),
                matches: false,
                operands: Vec::new(),
                results: Vec::new(),
            });
            continue;
        }
        // A signed bracketed result, `(-50)`.
        let right = right.trim_start_matches('(').trim_end_matches(')').trim();
        let Some((claimed, places, percent)) = number(right) else {
            continue;
        };
        // An opening bracket of the surrounding prose, as in `(2 + 3 = 5)`.
        let unopened = left
            .matches('(')
            .count()
            .saturating_sub(left.matches(')').count());
        let mut links: Vec<&str> = left
            .split('=')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        for _ in 0..unopened {
            links[0] = links[0].strip_prefix('(').unwrap_or(links[0]).trim_start();
        }
        let Some(values) = links
            .iter()
            .map(|l| evaluate(l))
            .collect::<Option<Vec<_>>>()
        else {
            // Every character `CLAIM` admits is a token, so only brackets
            // that do not pair within a step leave it unreadable.
            out.push(Calculation {
                text,
                computed: "could not read: a bracket does not pair within its step between `=` signs, so the gate cannot tell what it encloses; write the calculation on its own".into(),
                matches: false,
                operands: Vec::new(),
                results: Vec::new(),
            });
            continue;
        };
        let Some(values) = values.into_iter().collect::<Option<Vec<Q>>>() else {
            out.push(Calculation {
                text,
                computed: "undefined (division by zero)".into(),
                matches: false,
                operands: Vec::new(),
                results: Vec::new(),
            });
            continue;
        };
        let last = values.len() - 1;
        // Whether link `i + 1` starts from the value of link `i`, as a
        // running total writes it.
        let running: Vec<bool> = (0..last)
            .map(|i| {
                LEADING
                    .find(links[i + 1])
                    .and_then(|m| number(m.as_str()))
                    .is_some_and(|(q, p, pc)| close(&values[i], &q, p, pc))
            })
            .collect();
        let failed = (0..=last).find(|&i| {
            if i == last {
                return !close(&values[i], &claimed, places, percent);
            }
            // The whole next expression, to the finer of its own written
            // precision and the final result's.
            let written = NUMBER
                .find_iter(links[i + 1])
                .filter_map(|m| number(m.as_str()))
                .map(|(_, p, _)| p)
                .max()
                .unwrap_or(0);
            !running[i] && !close(&values[i], &values[i + 1], places.max(written), percent)
        });
        // A result in the unit of the expression it ends.
        let result = |written: &str, unit: Unit| {
            if unit == Unit::Dollars && !written.starts_with('$') && !written.ends_with('%') {
                format!("${written}")
            } else {
                written.to_string()
            }
        };
        let mut operands = Vec::new();
        let mut results = Vec::new();
        let mut unit = Unit::Unknown;
        for (i, link) in links.iter().enumerate() {
            let carried = (i > 0 && running[i - 1]).then_some(unit);
            let mut numbers = NUMBER.find_iter(link).map(|m| m.as_str());
            if let Some(carried) = carried {
                results.extend(numbers.next().map(|n| result(n, carried)));
            }
            operands.extend(numbers.map(str::to_string));
            unit = parse(link, carried)
                .flatten()
                .map_or(Unit::Unknown, |(_, u)| u);
        }
        results.extend(NUMBER.find(right).map(|m| result(m.as_str(), unit)));
        let shown = failed.unwrap_or(last);
        let percent = percent && shown == last;
        let value = if percent {
            values[shown].mul(&Q::new(BigInt::from(100), BigInt::from(1)))
        } else {
            values[shown].clone()
        };
        out.push(Calculation {
            text,
            computed: format!(
                "{}{}",
                decimal(&value, places.max(2)),
                if percent { "%" } else { "" }
            ),
            matches: failed.is_none(),
            operands,
            results,
        });
    }
    (out, unscanned)
}

/// A line without Markdown emphasis, heading marks or list markers. A marker
/// counts only when whitespace follows it, so `-5` keeps its sign.
fn normalise(line: &str) -> String {
    let text = line.replace("**", "").replace('`', "");
    let mut rest = text.trim();
    loop {
        let hashes = rest.trim_start_matches('#');
        let after = if hashes.len() < rest.len() {
            hashes
        } else {
            rest.strip_prefix(['-', '*', '+', '>']).unwrap_or(rest)
        };
        if after.len() == rest.len()
            || !(after.is_empty() || after.starts_with(char::is_whitespace))
        {
            return rest.to_string();
        }
        rest = after.trim_start();
    }
}

/// The contract section a line heads, ignoring a trailing colon and a
/// numbering prefix such as `1.`.
fn heading_of(line: &str) -> Option<&'static str> {
    let line = line.trim_end_matches(':').trim();
    let line = NUMBERED.find(line).map_or(line, |m| &line[m.end()..]);
    MANDATORY
        .iter()
        .chain(WHERE_MATERIAL.iter())
        .find(|h| line.eq_ignore_ascii_case(h))
        .copied()
}

/// `None` when the section is missing, otherwise whether any line before the
/// next contract section has content.
fn section(lines: &[String], heading: &str) -> Option<bool> {
    let start = lines.iter().position(|l| heading_of(l) == Some(heading))?;
    Some(
        lines[start + 1..]
            .iter()
            .take_while(|l| heading_of(l).is_none())
            // A Markdown rule such as `---` or `***` is not content.
            .any(|l| {
                !l.chars()
                    .all(|c| matches!(c, '-' | '*' | '_' | '=') || c.is_whitespace())
            }),
    )
}

/// The gate's findings for one response.
#[derive(Default)]
pub struct Report {
    pub status: Option<String>,
    pub blocks: Vec<String>,
    pub reviews: Vec<String>,
    pub notes: Vec<String>,
    pub calculations: Vec<Calculation>,
}

impl Report {
    pub fn verdict(&self) -> (&'static str, u8) {
        if !self.blocks.is_empty() {
            ("BLOCK", EXIT_BLOCK)
        } else if !self.reviews.is_empty() {
            ("REVIEW", EXIT_REVIEW)
        } else {
            ("PASS", EXIT_PASS)
        }
    }
}

fn check_fields(lines: &[String], stop: bool, report: &mut Report) {
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in lines {
        if let Some(c) = FIELD.captures(line) {
            fields.push((
                c[1].to_lowercase(),
                py::strip(&c[2]).trim_end_matches('.').to_string(),
            ));
        }
    }
    let values = |key: &str| {
        fields
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
    };
    let authority = values("action_authority");
    if authority.is_empty() {
        report
            .blocks
            .push("missing field: action_authority: NONE".into());
    }
    for value in authority.iter().filter(|v| **v != "NONE") {
        report.blocks.push(format!(
            "action_authority must be NONE, found {}",
            py::repr_str(value)
        ));
    }
    match values("state_changing_tool_action").as_slice() {
        [] => report
            .blocks
            .push("missing field: state_changing_tool_action".into()),
        values if values.iter().any(|v| v.is_empty()) => {
            report.blocks.push(
                "state_changing_tool_action is empty; write NONE or the record reference".into(),
            );
        }
        values => {
            for value in values.iter().filter(|v| **v != "NONE") {
                report.reviews.push(format!(
                    "state_changing_tool_action records {}; a person must confirm it against the external-action record",
                    py::repr_str(value)
                ));
            }
        }
    }
    for (key, label) in [
        ("historical authority", "Historical authority"),
        ("current duties", "Current duties"),
    ] {
        match values(key).as_slice() {
            [] if stop => report.notes.push(format!(
                "no {label} line; allowed only in an abbreviated escalation record"
            )),
            [] => report
                .blocks
                .push(format!("missing source-version line: {label}")),
            values => {
                // An allowed value, optionally followed by a note: `CONFIRMED (date)`.
                let allowed = |v: &str| {
                    SOURCE_VALUES.iter().any(|s| {
                        v.strip_prefix(s).is_some_and(|rest| {
                            !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                        })
                    })
                };
                for value in values.iter().filter(|v| !allowed(v)) {
                    report.blocks.push(format!(
                        "{label} must be CONFIRMED or SOURCE CURRENCY NOT CONFIRMED, found {}",
                        py::repr_str(value)
                    ));
                }
            }
        }
    }
}

/// Check one response against the output contract and its own arithmetic.
pub fn check(response: &str, conclusions: &[Conclusion]) -> Report {
    let mut report = Report::default();
    let lines: Vec<String> = py::splitlines(response)
        .into_iter()
        .map(normalise)
        .collect();
    let first = lines
        .iter()
        .find(|l| !l.is_empty())
        .map_or("", String::as_str);
    if !BANNER.is_match(first) {
        report
            .blocks
            .push("the first line must be the banner DRAFT — PROFESSIONAL REVIEW REQUIRED".into());
    }
    let statuses = harness::decision_statuses(response);
    match statuses.as_slice() {
        [one] if FULL_STATUSES.contains(&one.as_str()) || STOP_STATUSES.contains(&one.as_str()) => {
            report.status = Some(one.clone());
        }
        [one] if !one.is_empty() => report.blocks.push(format!(
            "{} is not a high-impact decision status; this gate applies to high-impact drafts",
            py::repr_str(one)
        )),
        [] | [_] => report
            .blocks
            .push("no primary decision status selects one allowed status".into()),
        _ => report.blocks.push(format!(
            "{} decision fields; write exactly one 'Primary decision status:' line",
            statuses.len()
        )),
    }
    let stop = report
        .status
        .as_deref()
        .is_some_and(|s| STOP_STATUSES.contains(&s));
    check_fields(&lines, stop, &mut report);
    for heading in MANDATORY {
        let problem = match section(&lines, heading) {
            Some(true) => continue,
            Some(false) => format!("empty {heading} section"),
            None => format!("no {heading} section"),
        };
        if stop && heading != "External-action record" {
            report.notes.push(format!(
                "{problem}; allowed only in an abbreviated escalation record"
            ));
        } else {
            report
                .blocks
                .push(format!("missing mandatory section: {heading} ({problem})"));
        }
    }
    for heading in WHERE_MATERIAL.iter().filter(|_| !stop) {
        let problem = match section(&lines, heading) {
            Some(true) => continue,
            Some(false) => "empty",
            None => "no",
        };
        report.notes.push(format!(
            "{problem} {heading} section; required where material"
        ));
    }
    let (found, unscanned) = calculations(response);
    report.calculations = found;
    for c in report.calculations.iter().filter(|c| !c.matches) {
        report.blocks.push(format!(
            "calculation does not reperform: {} (computes to {})",
            c.text, c.computed
        ));
    }
    for number in unscanned {
        report.reviews.push(format!(
            "calculation check: gave up scanning line {number} for calculations; a person must check its arithmetic"
        ));
    }
    for label in harness::prohibited_hits(response, conclusions) {
        report.reviews.push(format!("prohibited-conclusion screen: {label}; a human must confirm whether it is concluded or refused"));
    }
    report
}

/// A written amount: `$1,100.50`, `1100.5`, `12%`, `-$500`. Thousands are
/// grouped with commas or not at all; `$` and `%` do not combine.
static AMOUNT_TEXT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[-−]?\$?(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?%?$").expect("pattern")
});
/// Anything in a draft that looks like a dollar amount or a percentage,
/// malformed or not, taken whole: whitespace after `$` or before `%`, a
/// leading decimal point, thousands grouped with whitespace and a following
/// scale word. `\s` is Unicode whitespace, so a run of spaces, a no-break
/// space or a thin space counts in each position. Only `AMOUNT_TEXT` is
/// readable, so any other form is reported, never matched by a prefix such
/// as `$1.2` of `$1.2 million`. The `regex` crate runs in linear time and its
/// search cannot fail.
static DRAFT_AMOUNT: LazyLock<regex::Regex> = LazyLock::new(|| {
    let number = r"\.?\d(?:[\d.,]*\d)?(?:\s+\d{3})*";
    regex::Regex::new(&format!(
        r"(?i)\$\s*{number}(?:\s*%)?(?:\s*(?:thousand|million|billion|bn|k|m)\b)?|{number}\s*%"
    ))
    .expect("pattern")
});

/// An amount's magnitude, and whether it is a percentage. A percentage
/// matches only a percentage; a dollar amount and a plain number match each
/// other.
#[derive(Clone)]
struct Amount {
    value: Q,
    percent: bool,
}

impl Amount {
    fn read(text: &str) -> Option<Amount> {
        if !AMOUNT_TEXT.is_match(text) || (text.contains('$') && text.ends_with('%')) {
            return None;
        }
        let (q, _, percent) = number(text)?;
        Some(Amount {
            value: Q::new(q.n.abs(), q.d),
            percent,
        })
    }
    fn same(&self, other: &Amount) -> bool {
        self.percent == other.percent
            && &self.value.n * &other.value.d == &other.value.n * &self.value.d
    }
}

/// One supplied fact: its name in the facts file and its amount.
pub struct Fact {
    name: String,
    amount: Amount,
}

/// Read a facts file: a JSON object whose only key, `facts`, maps each fact's
/// name to its amount written as text, such as `"$1,100.50"`, `"1100.5"` or
/// `"10%"`. A JSON number is refused, so no amount passes through binary
/// floating point.
pub fn load_facts(text: &str) -> Result<Vec<Fact>, String> {
    let fail = |m: String| format!("facts file: {m}");
    let payload = pyjson::loads(text).map_err(|e| fail(format!("not valid JSON ({e})")))?;
    let Json::Object(top) = &payload else {
        return Err(fail("top level must be an object".into()));
    };
    if let Some((key, _)) = top.iter().find(|(k, _)| k != "facts") {
        return Err(fail(format!(
            "unknown key {}; the only key is 'facts'",
            py::repr_str(key)
        )));
    }
    let pairs = match payload.get("facts") {
        Some(Json::Object(pairs)) if !pairs.is_empty() => pairs,
        _ => return Err(fail("'facts' must map names to amounts".into())),
    };
    pairs
        .iter()
        .map(|(name, value)| {
            let written = value.as_str().ok_or_else(|| {
                fail(format!(
                    "{} must be an amount written as text, such as \"$1,100.50\", \"1100.5\" or \"10%\"; found {}",
                    py::repr_str(name),
                    value.repr()
                ))
            })?;
            let amount = Amount::read(written).ok_or_else(|| {
                fail(format!(
                    "{} is not an amount the gate can read: {}",
                    py::repr_str(name),
                    py::repr_str(written)
                ))
            })?;
            Ok(Fact {
                name: name.clone(),
                amount,
            })
        })
        .collect()
}

/// Flag each dollar amount or percentage in the draft that is neither a
/// supplied fact nor the result of a reperformed calculation whose dollar
/// and percentage inputs are facts or results of earlier calculations, and
/// for each amount equal to a fact, note that fact by name; an amount equal
/// to 2 or more facts is noted as ambiguous. This check adds reviews
/// and notes only, never a block: it finds amounts it cannot trace, and
/// cannot show that a traced amount is right or came from the fact it equals.
pub fn check_facts(response: &str, facts: &[Fact], report: &mut Report) {
    let mut known: Vec<Amount> = facts.iter().map(|f| f.amount.clone()).collect();
    let is_known = |known: &[Amount], a: &Amount| known.iter().any(|k| k.same(a));
    for c in report.calculations.iter().filter(|c| c.matches) {
        // Plain numbers such as `11` or `12` months are not amounts.
        let inputs_known = c
            .operands
            .iter()
            .filter(|t| t.contains('$') || t.ends_with('%'))
            .all(|t| Amount::read(t).is_some_and(|a| is_known(&known, &a)));
        if inputs_known {
            known.extend(
                c.results
                    .iter()
                    .filter(|t| t.contains('$') || t.ends_with('%'))
                    .filter_map(|t| Amount::read(t)),
            );
        }
    }
    let mut untraced: Vec<String> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    let mut equal: Vec<String> = Vec::new();
    for line in py::splitlines(response).into_iter().map(normalise) {
        let mut at = 0;
        while let Some(m) = DRAFT_AMOUNT.find_at(&line, at) {
            // A percentage inside a word or a longer number, as `x12%` or the
            // `10%` of a skipped `x1,10%`; a list such as `5%,10%` is read.
            // Look again from the next character, so `ab1 200%` still finds
            // `200%`.
            let mut previous = line[..m.start()].chars().rev();
            let before = previous.next();
            let inside = before.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
                || (matches!(before, Some('.' | ','))
                    && previous.next().is_some_and(|c| c.is_ascii_digit()));
            if !m.as_str().starts_with('$') && inside {
                at = m.start() + line[m.start()..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            at = m.end();
            let written = m.as_str();
            let (list, item) = match Amount::read(written) {
                None => (&mut unread, written.to_string()),
                Some(a) => {
                    let names: Vec<String> = facts
                        .iter()
                        .filter(|f| f.amount.same(&a))
                        .map(|f| py::repr_str(&f.name))
                        .collect();
                    match names.as_slice() {
                        [] if is_known(&known, &a) => continue,
                        [] => (&mut untraced, written.to_string()),
                        [name] => (
                            &mut equal,
                            format!("{written} is equal in amount to fact {name}"),
                        ),
                        [others @ .., last] => (
                            &mut equal,
                            format!(
                                "{written} is equal in amount to {} facts, {} and {last}; ambiguous, so the gate does not pick one",
                                names.len(),
                                others.join(", ")
                            ),
                        ),
                    }
                }
            };
            if !list.contains(&item) {
                list.push(item);
            }
        }
    }
    if !untraced.is_empty() {
        report.reviews.push(format!(
            "facts check: {} not in the facts file or derived by a reperformed calculation; a person must trace each to its source",
            untraced.join(", ")
        ));
    }
    if !unread.is_empty() {
        report.reviews.push(format!(
            "facts check: could not read {}; a person must check each",
            unread.join(", ")
        ));
    }
    report
        .notes
        .extend(equal.into_iter().map(|e| format!("facts check: {e}")));
    report.notes.push(
        "facts check: it flags amounts it cannot trace; an amount that matches a fact or a calculation is not thereby correct"
            .into(),
    );
}

/// The ruling series the authorities check reads: the public ruling types the
/// ATO names (TR, TD, LCR, GSTR, MT, CR, PR), GST determinations and
/// superannuation guarantee rulings, practical compliance guidelines, law
/// administration practice statements and TPB Guidance Statements.
const SERIES: &str = r"TR|TD|LCR|GSTR|GSTD|MT|CR|PR|SGR|PCG|PS\s+LA|TPB\(GS\)";
/// An Act named in full: capitalised words, connectives and bracketed
/// phrases ending in `Act` and its year, such as `A New Tax System (Goods and
/// Services Tax) Act 1999`. The year ends the word, so `Act 19970` and
/// `Act 1997a` name no Act.
const ACT: &str = r"[A-Z][\w'’-]*(?:\s+(?:[A-Z][\w'’-]*|and|of|for|the|to|on|in|an|a|\([^()]*\)))*\s+Act\s+\d{4}\b";
/// A word for a section or a subsection, in full or abbreviated, singular or
/// plural: `§`, `s`, `ss`, `sec`, `section`, `subs`, `sub-s`, `subsec`,
/// `sub-sec`, `subsection`. Each is spelt out with its `sub` prefix: `\b`
/// finds no word start before `section` in `subsection`, so `\bsection` alone
/// skips it.
const SECTION_WORD: &str = r"(?:§§?|\b(?i:(?:sub-?)?(?:ss?|secs?|sections?)))";
/// A numbered section or subsection reference, before any bracketed part.
/// `s` and `ss` need a full stop or a space before a number that is not
/// hyphenated, so `s8-1` is taken but a source label such as `S3` or `[S1]`
/// is not.
const SECTION_NUMBER: &str = r"(?:\b(?i:ss?)(?:\.\s*|\s+)\d[\w-]*|\b(?i:ss?)\d\w*-[\w-]*|(?:§§?|\b(?i:sub-?ss?|(?:sub-?)?secs?|(?:sub-?)?sections?))\.?\s*\d[\w-]*)";
/// A word for a paragraph or subparagraph: `para`, `paragraph`, `subpara`,
/// `subparagraph`, singular or plural.
const PARAGRAPH_WORD: &str = r"\b(?i:(?:sub-?)?para(?:graph)?s?)";
/// What may follow a word before its number or bracketed part: a full stop,
/// whitespace, both or neither, so `s. 8-1`, `s 8-1` and `s8-1` are alike.
const AFTER_WORD: &str = r"\.?\s*";
/// One bracketed part of a provision, such as `(1)` or `(a)`.
const PART: &str = r"\([0-9A-Za-z]+\)";
/// A list separator, including a comma followed by a conjunction.
const SEPARATOR: &str = r"(?:,\s*(?:(?i:and|or)\b\s*)?|(?i:and|or)\b\s*)";
/// Anything in a draft that looks like a ruling id or a reference to a
/// section, subsection, paragraph or subparagraph, taken whole with any
/// `of the <Act>` and any sibling parts that follow, such as `, (2) and (3)`,
/// so a citation the gate cannot read is reported rather than skipped. A
/// paragraph needs a bracketed part, a named Act or a hyphenated number
/// (`range`), which `check_authorities` reads only before `of the`, so
/// `paragraph 45` or `paragraphs 45-50 of TR 2006/10` of a ruling is not
/// taken. A part named without its section (`bare`), such as
/// `subsection (2)`, is taken alone; `check_authorities` passes over it when
/// it opens a composite citation that ends in a numbered one.
static AUTHORITY_TEXT: LazyLock<regex::Regex> = LazyLock::new(|| {
    let provision = format!(r"\d[\w-]*(?:{PART})*");
    // A named Act distinguishes compact section numbers from source labels
    // and unbracketed paragraphs from ruling paragraphs.
    let qualified = format!(
        r"(?:{SECTION_WORD}{AFTER_WORD}{provision}|{PARAGRAPH_WORD}{AFTER_WORD}{provision}(?:\s*{SEPARATOR}(?:{provision}|(?:{PART})+))*)\s+of\s+the\s+{ACT}"
    );
    regex::Regex::new(&format!(
        r"\b(?:{SERIES})\s*\d[\w/]*|(?:{qualified}|{SECTION_NUMBER}(?:{PART})*|{PARAGRAPH_WORD}{AFTER_WORD}(?:\d[\w-]*(?:{PART})+|(?P<range>\d\w*-[\w-]*)))(?:\s*{SEPARATOR}(?:{PART})+)*|(?P<bare>(?:{SECTION_WORD}|{PARAGRAPH_WORD}){AFTER_WORD}(?:{PART})+)"
    ))
    .expect("pattern")
});
/// The opening of a composite citation: parts named without their section,
/// each `of` the next, as in `paragraph (a) of ` before `subsection 9-5(1)`.
static COMPOSITE_OPENING: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(
        r"^(?:(?:{SECTION_WORD}|{PARAGRAPH_WORD}){AFTER_WORD}(?:{PART})+\s+of\s+(?:the\s+)?)+"
    ))
    .expect("pattern")
});
/// `of the` after a hyphenated paragraph number, which marks it as a
/// provision of an Act rather than a range of a ruling's paragraphs.
static OF_THE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\s+of\s+the\b").expect("pattern"));
/// A ruling named after a paragraph range is checked by its own citation.
static RULING_TAIL: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(
        r"^\s+of\s+the\s+(?:(?i:ruling)\s+)?(?:{SERIES})\s*\d"
    ))
    .expect("pattern")
});
/// A readable citation: a ruling id, whose first part has 2 or 4 digits (a
/// year before 1999 has 2), with an optional draft `D` before its number and
/// a suffix such as `W` or `A1`, or one section of a named Act, or one
/// subsection, paragraph or subparagraph of it with its bracketed part. A
/// plural such as `ss` or `subsections` may list more than the scanner takes,
/// so it is never read.
static AUTHORITY: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(
        r"^(?:({SERIES})\s+((?:\d{{2}}|\d{{4}})/D?\d{{1,4}}(?:[A-Z]+\d*)?)|(?:(?:§|(?i:s|sec|section)){AFTER_WORD}(\d+[A-Z]*(?:-\d+[A-Z]*)?(?:{PART})*)|(?i:sub-?(?:s|sec|section)|(?:sub-?)?para(?:graph)?){AFTER_WORD}(\d+[A-Z]*(?:-\d+[A-Z]*)?(?:{PART})+))\s+of\s+the\s+({ACT}))$"
    ))
    .expect("pattern")
});

/// One cited or listed authority, in the form it compares by: `s <section>
/// of the <Act>` or `<series> <id>`, with single spaces. A subsection or
/// paragraph keeps its bracketed parts: `subsection 8-1(1)` is `s 8-1(1)`.
#[derive(Debug, PartialEq)]
pub struct Authority(String);

impl Authority {
    pub(crate) fn read(text: &str) -> Option<Authority> {
        let c = AUTHORITY.captures(text)?;
        let words = |i: usize| c[i].split_whitespace().collect::<Vec<_>>().join(" ");
        Some(Authority(match c.get(1) {
            Some(_) => format!("{} {}", words(1), &c[2]),
            None => format!("s {} of the {}", c.get(3).or(c.get(4))?.as_str(), words(5)),
        }))
    }

    /// The citation without subsections, so `s 109E(6)` meets `s 109E`. A
    /// ruling id has no section and stays whole.
    pub(crate) fn section(&self) -> String {
        match self.0.split_once(" of the ") {
            Some((section, act)) => {
                let bare = section.split('(').next().unwrap_or(section);
                format!("{bare} of the {act}")
            }
            None => self.0.clone(),
        }
    }

    /// The citation and each provision containing it, narrowest first:
    /// `s 8-1(1)(a)`, `s 8-1(1)`, `s 8-1`. A ruling id contains nothing else.
    fn containing(&self) -> Vec<Authority> {
        let Some((mut provision, act)) = self.0.split_once(" of the ") else {
            return vec![Authority(self.0.clone())];
        };
        let mut found = vec![Authority(format!("{provision} of the {act}"))];
        while let Some(i) = provision.rfind('(') {
            provision = &provision[..i];
            found.push(Authority(format!("{provision} of the {act}")));
        }
        found
    }
}

/// Read an authorities file: a JSON object whose only key, `authorities`,
/// lists each accepted authority as text in a form the gate reads, such as
/// `"s 8-1 of the Income Tax Assessment Act 1997"` or `"TR 2006/10"`.
pub fn load_authorities(text: &str) -> Result<Vec<Authority>, String> {
    let fail = |m: String| format!("authorities file: {m}");
    let payload = pyjson::loads(text).map_err(|e| fail(format!("not valid JSON ({e})")))?;
    let Json::Object(top) = &payload else {
        return Err(fail("top level must be an object".into()));
    };
    if let Some((key, _)) = top.iter().find(|(k, _)| k != "authorities") {
        return Err(fail(format!(
            "unknown key {}; the only key is 'authorities'",
            py::repr_str(key)
        )));
    }
    let items = match payload.get("authorities") {
        Some(Json::Array(items)) if !items.is_empty() => items,
        _ => return Err(fail("'authorities' must list authorities".into())),
    };
    items
        .iter()
        .map(|value| {
            let written = value.as_str().ok_or_else(|| {
                fail(format!(
                    "each authority must be written as text, such as \"s 8-1 of the Income Tax Assessment Act 1997\" or \"TR 2006/10\"; found {}",
                    value.repr()
                ))
            })?;
            Authority::read(written).ok_or_else(|| {
                fail(format!(
                    "not an authority the gate can read: {}",
                    py::repr_str(written)
                ))
            })
        })
        .collect()
}

/// Whether a match of `AUTHORITY_TEXT` is a citation to check: not a
/// possessive such as `client's 2 trusts` (a quotation opening on `'s 8-1` is
/// still read), nor a hyphenated paragraph number without `of the` after it
/// or followed by an explicit ruling reference.
fn is_citation(text: &str, c: &regex::Captures) -> bool {
    let start = c.get(0).expect("whole match").start();
    let mut before = text[..start].chars().rev();
    let possessive = matches!(before.next(), Some('\'' | '’'))
        && before.next().is_some_and(char::is_alphanumeric);
    let ruling_range = c.name("range").is_some_and(|r| {
        let tail = &text[r.end()..];
        !OF_THE.is_match(tail) || RULING_TAIL.is_match(tail)
    });
    !possessive && !ruling_range
}

/// Whether the part named without its section at `start` opens a composite
/// citation, such as `paragraph (a) of subsection 9-5(1)`, that ends in a
/// numbered citation the scanner reads next. Anything else after the opening,
/// such as `paragraph 3 of Schedule 1`, leaves the part to be reported.
fn opens_composite(text: &str, start: usize) -> bool {
    let Some(opening) = COMPOSITE_OPENING.find(&text[start..]) else {
        return false;
    };
    let next = start + opening.end();
    AUTHORITY_TEXT.captures_at(text, next).is_some_and(|c| {
        c.get(0).is_some_and(|m| m.start() == next)
            && c.name("bare").is_none()
            && is_citation(text, &c)
    })
}

/// The sections, subsections, paragraphs and subparagraphs of a named Act,
/// and the ruling ids, a draft cites that the authorities list omits, then
/// the section-like or ruling-like text the gate cannot read whole, each in
/// the order first cited. A citation is listed when the list holds it or a
/// provision containing it, so `s 8-1(1)` lists `s 8-1(1)(a)` but neither
/// `s 8-1` nor `s 8-1(2)`.
pub(crate) fn unlisted_citations(
    response: &str,
    authorities: &[Authority],
) -> (Vec<String>, Vec<String>) {
    let mut unlisted: Vec<String> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    // One text, so a citation wrapped across lines (`TR` then `2019/7`) is
    // read whole; each line loses its Markdown marks first.
    let text = py::splitlines(response)
        .into_iter()
        .map(normalise)
        .collect::<Vec<_>>()
        .join("\n");
    for c in AUTHORITY_TEXT.captures_iter(&text) {
        let m = c.get(0).expect("whole match");
        if !is_citation(&text, &c)
            || (c.name("bare").is_some() && opens_composite(&text, m.start()))
        {
            continue;
        }
        let written = m.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
        let list = match Authority::read(&written) {
            None => &mut unread,
            Some(a) if a.containing().iter().any(|c| authorities.contains(c)) => continue,
            Some(_) => &mut unlisted,
        };
        if !list.contains(&written) {
            list.push(written);
        }
    }
    (unlisted, unread)
}

/// Flag each section of a named Act or ruling id the draft cites that is not
/// in the authorities list, and each section-like or ruling-like text the
/// gate cannot read. A citation is listed when the list holds it or a
/// provision containing it, so `s 8-1(1)` lists `s 8-1(1)(a)` but neither
/// `s 8-1` nor `s 8-1(2)`. This check adds reviews and a note only, never a block:
/// it cannot show that a listed authority is current or correctly applied.
pub fn check_authorities(response: &str, authorities: &[Authority], report: &mut Report) {
    let (unlisted, unread) = unlisted_citations(response, authorities);
    if !unlisted.is_empty() {
        report.reviews.push(format!(
            "authorities check: {} not in the authorities file; a person must check each against its source",
            unlisted.join(", ")
        ));
    }
    if !unread.is_empty() {
        report.reviews.push(format!(
            "authorities check: could not read {} as a section, subsection, paragraph or subparagraph of a named Act or a ruling id; a person must check each",
            unread.join(", ")
        ));
    }
    report.notes.push(
        "authorities check: it reads only a section of a named Act, or a subsection, paragraph or subparagraph of one, and a ruling id, so a pass does not show that every authority the draft cites was checked; a listed authority is not thereby current or correctly applied"
            .into(),
    );
}

/// The closing mark for an opening quotation mark.
fn closing_mark(opening: char) -> Option<char> {
    match opening {
        '"' | '\'' => Some(opening),
        '\u{201c}' => Some('\u{201d}'),
        '\u{2018}' => Some('\u{2019}'),
        _ => None,
    }
}

/// The quoted spans in a draft, and the opening of any quotation with no end.
/// A straight apostrophe after a letter or digit opens nothing, so `client's`
/// is not a quotation; any other mark there still opens one, so `said"a b"` is
/// read. A mark before whitespace opens nothing, so a stray mark cannot pair
/// with the opening of a real quotation. An apostrophe mark before a letter or
/// digit closes nothing.
fn quoted_spans(response: &str) -> (Vec<String>, Vec<String>) {
    let (mut spans, mut quoted, mut prose) = (Vec::new(), Vec::new(), Vec::new());
    for line in py::splitlines(response) {
        match line.trim_start().strip_prefix('>') {
            Some(text) => quoted.push(text.trim().to_string()),
            None => {
                if !quoted.is_empty() {
                    spans.push(quoted.join(" "));
                    quoted.clear();
                }
                prose.push(line);
            }
        }
    }
    if !quoted.is_empty() {
        spans.push(quoted.join(" "));
    }
    let chars: Vec<char> = prose.join("\n").chars().collect();
    let mut unclosed = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let apostrophe = chars[i] == '\'' && i > 0 && chars[i - 1].is_alphanumeric();
        let opens = !apostrophe && !chars.get(i + 1).is_some_and(|c| c.is_whitespace());
        let Some(close) = closing_mark(chars[i]).filter(|_| opens) else {
            i += 1;
            continue;
        };
        let apostrophe_like = close != '"' && close != '\u{201d}';
        let end = (i + 1..chars.len()).find(|&j| {
            chars[j] == close
                && !(apostrophe_like && chars.get(j + 1).is_some_and(|c| c.is_alphanumeric()))
        });
        match end {
            Some(end) => {
                spans.push(chars[i + 1..end].iter().collect());
                i = end + 1;
            }
            None => {
                unclosed.push(chars[i..chars.len().min(i + 40)].iter().collect());
                i += 1;
            }
        }
    }
    (spans, unclosed)
}

/// Flag each quoted span not found verbatim in a supplied source. This check
/// adds reviews and a note only, never a block: finding the words does not
/// show that the source applies or supports what the draft says.
pub fn check_quotes(response: &str, sources: &[String], report: &mut Report) {
    let (spans, unclosed) = quoted_spans(response);
    let status_of = |span: &str| -> Option<String> {
        let mut best = None;
        for source in sources {
            let result = quotecheck::assess_quote(source, span, None, true).ok()?;
            match result.get("status").and_then(Json::as_str) {
                Some("EXACT_MATCH") => return Some("EXACT_MATCH".to_string()),
                Some("NORMALISED_MATCH") => best = Some("NORMALISED_MATCH".to_string()),
                _ => {}
            }
        }
        best
    };
    let mut found = 0;
    for span in spans {
        if span.split_whitespace().count() < 2 {
            continue;
        }
        let written = py::repr_str(&span);
        match status_of(span.trim().trim_end_matches(['.', ',', ';', ':'])).as_deref() {
            Some("EXACT_MATCH") => found += 1,
            Some(_) => report.reviews.push(format!(
                "quotes check: {written} matches a supplied source only after normalising, so it is not verbatim; a person must check it"
            )),
            None => report.reviews.push(format!(
                "quotes check: {written} is not in any supplied source; a person must check it"
            )),
        }
    }
    for start in unclosed {
        report.reviews.push(format!(
            "quotes check: could not find the end of the quotation starting {}; a person must check it",
            py::repr_str(&start)
        ));
    }
    report.notes.push(format!(
        "quotes check: {found} quoted span(s) found verbatim in the supplied sources; that is text membership only, not support for the claim, and a single quoted word is not read"
    ));
}

fn report_json(report: &Report) -> Json {
    let list = |items: &[String]| Json::Array(items.iter().map(|s| Json::str_obj(s)).collect());
    let (verdict, _) = report.verdict();
    Json::Object(vec![
        ("verdict".into(), Json::str_obj(verdict)),
        (
            "status".into(),
            report.status.as_deref().map_or(Json::Null, Json::str_obj),
        ),
        ("blocks".into(), list(&report.blocks)),
        ("reviews".into(), list(&report.reviews)),
        ("notes".into(), list(&report.notes)),
        (
            "calculations".into(),
            Json::Array(
                report
                    .calculations
                    .iter()
                    .map(|c| {
                        Json::Object(vec![
                            ("text".into(), Json::str_obj(&c.text)),
                            ("computed".into(), Json::str_obj(&c.computed)),
                            ("matches".into(), Json::Bool(c.matches)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

#[derive(clap::Args)]
pub struct Args {
    /// The model response to check, or - for standard input.
    response: String,
    /// Print the findings as JSON.
    #[arg(long)]
    json: bool,
    /// A JSON facts file; each dollar amount or percentage in the draft that
    /// is neither a fact nor a shown calculation's result asks for review.
    #[arg(long)]
    facts: Option<String>,
    /// A JSON authorities file; each section of a named Act or ruling id the
    /// draft cites that is not listed asks for review. Other citations are
    /// not checked.
    #[arg(long)]
    authorities: Option<String>,
    /// A UTF-8 text file the draft may quote; repeat for each source. Each
    /// quoted span not found verbatim in one asks for review.
    #[arg(long = "source")]
    sources: Vec<String>,
}

fn read_response(path: &str) -> Result<String, Fail> {
    if path == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
            .map_err(|e| Fail::Os(e.to_string()))?;
        Ok(text)
    } else {
        builder::read_text(&PyPath::new(path))
    }
}

/// Read and parse the `--facts` or `--authorities` file, when one is given.
pub(crate) fn load_list<T>(
    path: Option<&str>,
    what: &str,
    parse: fn(&str) -> Result<T, String>,
) -> Result<Option<T>, String> {
    path.map(|path| {
        builder::read_text(&PyPath::new(path))
            .map_err(|e| format!("{what} file: {e}"))
            .and_then(|text| parse(&text))
    })
    .transpose()
}

pub fn main(root: Option<&str>, args: Args) -> u8 {
    let loaded = crate::root_for(root)
        .and_then(|root| builder::load_sources(&root))
        .and_then(|sources| Ok((sources, read_response(&args.response)?)));
    let (sources, response) = match loaded {
        Ok(found) => found,
        Err(e) => {
            errln!("gate: {e}");
            return EXIT_COULD_NOT_RUN;
        }
    };
    let lists = load_list(args.facts.as_deref(), "facts", load_facts).and_then(|facts| {
        let authorities = load_list(args.authorities.as_deref(), "authorities", load_authorities)?;
        let quoted = args
            .sources
            .iter()
            .map(|path| {
                builder::read_text(&PyPath::new(path)).map_err(|e| format!("source file: {e}"))
            })
            .collect::<Result<Vec<String>, String>>()?;
        Ok((facts, authorities, quoted))
    });
    let (facts, authorities, quoted) = match lists {
        Ok(lists) => lists,
        Err(e) => {
            errln!("gate: {e}");
            return EXIT_COULD_NOT_RUN;
        }
    };
    let mut report = check(&response, &sources.whole_response.prohibited_conclusions);
    if let Some(facts) = &facts {
        check_facts(&response, facts, &mut report);
    }
    if let Some(authorities) = &authorities {
        check_authorities(&response, authorities, &mut report);
    }
    if !quoted.is_empty() {
        check_quotes(&response, &quoted, &mut report);
    }
    let (verdict, code) = report.verdict();
    if args.json {
        outln!("{}", pyjson::dumps(&report_json(&report), false));
        return code;
    }
    outln!(
        "gate: {verdict}{}",
        report
            .status
            .as_deref()
            .map_or(String::new(), |s| format!(" ({s})"))
    );
    for (label, items) in [
        ("block", &report.blocks),
        ("review", &report.reviews),
        ("note", &report.notes),
    ] {
        for item in items {
            outln!("  {label}: {item}");
        }
    }
    for c in report.calculations.iter().filter(|c| c.matches) {
        outln!("  reperformed: {}", c.text);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_rejects_unknown_operators_and_missing_operands() {
        for expression in ["2 ^ 3", "2 + )"] {
            assert!(evaluate(expression).is_none(), "{expression}");
        }
    }

    #[test]
    fn a_ruling_has_no_subsection_to_strip() {
        let authority = Authority::read("TR 2020/1").unwrap();
        assert_eq!(authority.section(), "TR 2020/1");
    }
}
