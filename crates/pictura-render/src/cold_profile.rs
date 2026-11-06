//! Cold-composite diagnostics: the line-oriented report schema and the
//! deterministic baseline/candidate comparator.
//!
//! A profile child writes one report per process. A report records one value
//! per metric, the cohort identity, optional nested phase timings, and output
//! digests. Comparison requires at least three fresh-process reports per
//! revision and uses doubled integer medians so even sample counts never round.
//!
//! Run `cold_compare <baseline-file-or-dir> <candidate-file-or-dir>`; it exits
//! 0 improved, 2 regressed, 3 inconclusive, 4 insufficient, 5 invalid, and 1 on
//! usage or I/O failure.

use std::collections::BTreeMap;
use std::path::Path;

/// Protocol identity written into every report.
pub const PROTOCOL: &str = "cold-composite-responsiveness";
/// Protocol version.
pub const VERSION: u32 = 1;

/// Primary metric: cold end-to-end time in nanoseconds.
pub const PRIMARY: &str = "primary";
/// Steady-state control: repeated full render/edit median in nanoseconds.
pub const REPEATED_FULL: &str = "repeated_full";
/// Steady-state control: warm bounded-region time in nanoseconds.
pub const REGION: &str = "region";
/// Metrics every report must carry.
pub const REQUIRED: [&str; 3] = [PRIMARY, REPEATED_FULL, REGION];

/// The fixed 10% noise policy, expressed as `20 * range > median2`.
const NOISE_SCALE: u128 = 20;

/// One process's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Protocol identity.
    pub protocol: String,
    /// Protocol version.
    pub version: u32,
    /// Revision, reported but allowed to differ between cohorts.
    pub revision: String,
    /// Cohort identity that comparison must match.
    pub cohort: Cohort,
    /// One integer-nanosecond value per metric.
    pub metrics: BTreeMap<String, u64>,
    /// Coarse phase timings, keyed by label.
    pub phases: BTreeMap<String, PhaseReport>,
    /// Output digests, keyed by name.
    pub digests: BTreeMap<String, String>,
}

/// Cohort identity that both revisions must share.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cohort {
    /// `engine` or `application`.
    pub mode: String,
    /// Opaque input id; never a path or layer name.
    pub input: String,
    /// Opaque edit recipe id, including direction.
    pub edit_recipe: String,
    /// Opaque machine id.
    pub machine: String,
    /// Build configuration summary.
    pub build: String,
    /// Fixed worker count.
    pub threads: u32,
    /// Requested backend.
    pub requested_backend: String,
    /// Executed backend.
    pub executed_backend: String,
    /// Fallback reason when the executed backend differs from the request.
    pub fallback: Option<String>,
    /// `on` or `off`.
    pub metrics: String,
}

/// One coarse phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseReport {
    /// Accumulated nanoseconds.
    pub total_ns: u64,
    /// Number of recorded spans.
    pub count: u64,
    /// Whether this span is contained in another reported span.
    pub nested: bool,
}

impl Report {
    /// An empty report for `cohort`.
    pub fn new(cohort: Cohort) -> Self {
        Self {
            protocol: PROTOCOL.to_string(),
            version: VERSION,
            revision: String::new(),
            cohort,
            metrics: BTreeMap::new(),
            phases: BTreeMap::new(),
            digests: BTreeMap::new(),
        }
    }

    /// Record one metric in nanoseconds.
    pub fn set_metric(&mut self, name: &str, ns: u64) {
        self.metrics.insert(name.to_string(), ns);
    }

    /// Record one coarse phase.
    pub fn set_phase(&mut self, label: &str, phase: PhaseReport) {
        self.phases.insert(label.to_string(), phase);
    }

    /// Record one output digest.
    pub fn set_digest(&mut self, name: &str, value: impl Into<String>) {
        self.digests.insert(name.to_string(), value.into());
    }

    /// Render the line-oriented report.
    pub fn render(&self) -> String {
        let mut out = String::new();
        push(&mut out, "protocol", &self.protocol);
        push(&mut out, "version", &self.version.to_string());
        push(&mut out, "revision", &self.revision);
        push(&mut out, "cohort.mode", &self.cohort.mode);
        push(&mut out, "cohort.input", &self.cohort.input);
        push(&mut out, "cohort.edit_recipe", &self.cohort.edit_recipe);
        push(&mut out, "cohort.machine", &self.cohort.machine);
        push(&mut out, "cohort.build", &self.cohort.build);
        push(&mut out, "cohort.threads", &self.cohort.threads.to_string());
        push(
            &mut out,
            "cohort.backend.requested",
            &self.cohort.requested_backend,
        );
        push(
            &mut out,
            "cohort.backend.executed",
            &self.cohort.executed_backend,
        );
        if let Some(fallback) = &self.cohort.fallback {
            push(&mut out, "cohort.backend.fallback", fallback);
        }
        push(&mut out, "cohort.metrics", &self.cohort.metrics);
        for (name, ns) in &self.metrics {
            push(&mut out, &format!("metric.{name}"), &ns.to_string());
        }
        for (label, phase) in &self.phases {
            push(
                &mut out,
                &format!("phase.{label}.total_ns"),
                &phase.total_ns.to_string(),
            );
            push(
                &mut out,
                &format!("phase.{label}.count"),
                &phase.count.to_string(),
            );
            push(
                &mut out,
                &format!("phase.{label}.nested"),
                if phase.nested { "true" } else { "false" },
            );
        }
        for (name, value) in &self.digests {
            push(&mut out, &format!("digest.{name}"), value);
        }
        out
    }

    /// Parse a line-oriented report.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut protocol = None;
        let mut version = None;
        let mut revision = String::new();
        let mut cohort = Cohort::default();
        let mut metrics = BTreeMap::new();
        let mut phases: BTreeMap<String, PhaseReport> = BTreeMap::new();
        let mut digests = BTreeMap::new();

        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: expected key=value", index + 1))?;
            match key {
                "protocol" => protocol = Some(value.to_string()),
                "version" => {
                    version = Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| format!("line {}: invalid version", index + 1))?,
                    )
                }
                "revision" => revision = value.to_string(),
                "cohort.mode" => cohort.mode = value.to_string(),
                "cohort.input" => cohort.input = value.to_string(),
                "cohort.edit_recipe" => cohort.edit_recipe = value.to_string(),
                "cohort.machine" => cohort.machine = value.to_string(),
                "cohort.build" => cohort.build = value.to_string(),
                "cohort.threads" => {
                    cohort.threads = value
                        .parse::<u32>()
                        .map_err(|_| format!("line {}: invalid thread count", index + 1))?
                }
                "cohort.backend.requested" => cohort.requested_backend = value.to_string(),
                "cohort.backend.executed" => cohort.executed_backend = value.to_string(),
                "cohort.backend.fallback" => cohort.fallback = Some(value.to_string()),
                "cohort.metrics" => cohort.metrics = value.to_string(),
                _ => {
                    if let Some(name) = key.strip_prefix("metric.") {
                        metrics.insert(name.to_string(), parse_ns(value, index)?);
                    } else if let Some(rest) = key.strip_prefix("phase.") {
                        let (label, field) = rest.rsplit_once('.').ok_or_else(|| {
                            format!("line {}: expected phase.<label>.<field>", index + 1)
                        })?;
                        let phase = phases.entry(label.to_string()).or_default();
                        match field {
                            "total_ns" => phase.total_ns = parse_ns(value, index)?,
                            "count" => phase.count = parse_ns(value, index)?,
                            "nested" => {
                                phase.nested = value.parse::<bool>().map_err(|_| {
                                    format!("line {}: invalid nested flag", index + 1)
                                })?
                            }
                            _ => return Err(format!("line {}: unknown phase field", index + 1)),
                        }
                    } else if let Some(name) = key.strip_prefix("digest.") {
                        digests.insert(name.to_string(), value.to_string());
                    }
                }
            }
        }

        let protocol = protocol.ok_or("missing protocol")?;
        let version = version.ok_or("missing version")?;
        for name in REQUIRED {
            if !metrics.contains_key(name) {
                return Err(format!("missing metric {name}"));
            }
        }
        if cohort.input.is_empty() {
            return Err("missing cohort.input".to_string());
        }
        Ok(Self {
            protocol,
            version,
            revision,
            cohort,
            metrics,
            phases,
            digests,
        })
    }

    /// Read and parse a report file.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        Self::parse(&text)
    }

    /// Render and write a report file.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.render())
            .map_err(|error| format!("write {}: {error}", path.display()))
    }
}

fn push(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push_str(&value.replace(['\n', '\r'], " "));
    out.push('\n');
}

fn parse_ns(value: &str, index: usize) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("line {}: expected integer nanoseconds", index + 1))
}

/// One metric's evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricComparison {
    /// Metric name.
    pub name: String,
    /// Baseline per-process values.
    pub baseline: Vec<u64>,
    /// Candidate per-process values.
    pub candidate: Vec<u64>,
    /// Baseline doubled median.
    pub baseline_median2: u128,
    /// Candidate doubled median.
    pub candidate_median2: u128,
    /// Baseline max minus min.
    pub baseline_range: u64,
    /// Candidate max minus min.
    pub candidate_range: u64,
    /// `max(range_b, range_c)`.
    pub v: u64,
    /// Whether the batch exceeded the fixed noise policy.
    pub noisy: bool,
}

/// Comparison outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Primary improved and no required control regressed.
    Improved,
    /// Primary or a required control regressed.
    Regressed,
    /// No deterministic decision.
    Inconclusive,
    /// Fewer than three valid samples per revision.
    Insufficient,
    /// Malformed or mismatched input.
    Invalid,
}

impl Outcome {
    /// Process exit code for comparison mode.
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Improved => 0,
            Self::Regressed => 2,
            Self::Inconclusive => 3,
            Self::Insufficient => 4,
            Self::Invalid => 5,
        }
    }

    /// Stable lowercase name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Improved => "improved",
            Self::Regressed => "regressed",
            Self::Inconclusive => "inconclusive",
            Self::Insufficient => "insufficient",
            Self::Invalid => "invalid",
        }
    }
}

/// A comparison result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Outcome.
    pub outcome: Outcome,
    /// Human-readable reason.
    pub reason: String,
    /// Per-metric evidence.
    pub metrics: Vec<MetricComparison>,
}

impl Comparison {
    /// Render the evidence for a terminal.
    pub fn render(&self) -> String {
        let mut out = format!(
            "outcome={} exit={} reason={}\n",
            self.outcome.name(),
            self.outcome.exit_code(),
            self.reason
        );
        for metric in &self.metrics {
            out.push_str(&format!(
                "metric.{} baseline={:?} candidate={:?} baseline_median2={} candidate_median2={} baseline_range={} candidate_range={} v={} noisy={}\n",
                metric.name,
                metric.baseline,
                metric.candidate,
                metric.baseline_median2,
                metric.candidate_median2,
                metric.baseline_range,
                metric.candidate_range,
                metric.v,
                metric.noisy,
            ));
        }
        out
    }
}

/// Compare a baseline cohort against a candidate cohort.
pub fn compare(baseline: &[Report], candidate: &[Report]) -> Comparison {
    if baseline.is_empty() || candidate.is_empty() {
        return invalid("empty cohort");
    }
    let first = &baseline[0];
    for report in baseline.iter().chain(candidate) {
        if report.protocol != first.protocol || report.version != first.version {
            return invalid("protocol/version mismatch");
        }
        if report.cohort != first.cohort {
            return invalid("cohort mismatch");
        }
        for name in REQUIRED {
            if !report.metrics.contains_key(name) {
                return invalid(&format!("missing metric {name}"));
            }
        }
    }
    if baseline.len() < 3 || candidate.len() < 3 {
        return Comparison {
            outcome: Outcome::Insufficient,
            reason: "fewer than three independent samples per revision".to_string(),
            metrics: Vec::new(),
        };
    }

    let metrics: Vec<MetricComparison> = REQUIRED
        .iter()
        .map(|name| metric_comparison(name, baseline, candidate))
        .collect();
    let noisy = metrics.iter().any(|metric| metric.noisy);
    if noisy {
        return Comparison {
            outcome: Outcome::Inconclusive,
            reason: "noise limit exceeded".to_string(),
            metrics,
        };
    }

    let primary = &metrics[0];
    let regressed = metrics.iter().any(|metric| {
        metric.candidate_median2 > metric.baseline_median2
            && metric.candidate_median2 - metric.baseline_median2 > 2 * u128::from(metric.v)
    });
    if regressed {
        return Comparison {
            outcome: Outcome::Regressed,
            reason: "a required metric increased beyond 2*V".to_string(),
            metrics,
        };
    }
    if primary.baseline_median2 > primary.candidate_median2
        && primary.baseline_median2 - primary.candidate_median2 > 2 * u128::from(primary.v)
    {
        return Comparison {
            outcome: Outcome::Improved,
            reason: "primary decreased beyond 2*V with no regressed control".to_string(),
            metrics,
        };
    }
    Comparison {
        outcome: Outcome::Inconclusive,
        reason: "change does not exceed 2*V".to_string(),
        metrics,
    }
}

fn invalid(reason: &str) -> Comparison {
    Comparison {
        outcome: Outcome::Invalid,
        reason: reason.to_string(),
        metrics: Vec::new(),
    }
}

fn metric_comparison(name: &str, baseline: &[Report], candidate: &[Report]) -> MetricComparison {
    let b: Vec<u64> = baseline.iter().map(|r| r.metrics[name]).collect();
    let c: Vec<u64> = candidate.iter().map(|r| r.metrics[name]).collect();
    let baseline_median2 = median2(&b);
    let candidate_median2 = median2(&c);
    let baseline_range = range(&b);
    let candidate_range = range(&c);
    let v = baseline_range.max(candidate_range);
    let noisy = NOISE_SCALE * u128::from(baseline_range) > baseline_median2
        || NOISE_SCALE * u128::from(candidate_range) > candidate_median2
        || (baseline_median2 == 0 && baseline_range > 0)
        || (candidate_median2 == 0 && candidate_range > 0);
    MetricComparison {
        name: name.to_string(),
        baseline: b,
        candidate: c,
        baseline_median2,
        candidate_median2,
        baseline_range,
        candidate_range,
        v,
        noisy,
    }
}

fn median2(values: &[u64]) -> u128 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    if len % 2 == 1 {
        2 * u128::from(sorted[len / 2])
    } else {
        u128::from(sorted[len / 2 - 1]) + u128::from(sorted[len / 2])
    }
}

fn range(values: &[u64]) -> u64 {
    values
        .iter()
        .max()
        .copied()
        .unwrap_or(0)
        .saturating_sub(values.iter().min().copied().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cohort() -> Cohort {
        Cohort {
            mode: "engine".to_string(),
            input: "generated-nested-v1".to_string(),
            edit_recipe: "opacity-a-to-b".to_string(),
            machine: "test".to_string(),
            build: "debug".to_string(),
            threads: 4,
            requested_backend: "cpu".to_string(),
            executed_backend: "cpu".to_string(),
            fallback: None,
            metrics: "off".to_string(),
        }
    }

    fn report(primary: u64, repeated_full: u64, region: u64) -> Report {
        let mut report = Report::new(cohort());
        report.set_metric(PRIMARY, primary);
        report.set_metric(REPEATED_FULL, repeated_full);
        report.set_metric(REGION, region);
        report
    }

    fn reports(values: &[(u64, u64, u64)]) -> Vec<Report> {
        values
            .iter()
            .map(|(primary, repeated, region)| report(*primary, *repeated, *region))
            .collect()
    }

    #[test]
    fn cold_profile_reports_first_sample_and_nested_phases() {
        let mut report = report(15_000, 243, 138);
        report.set_metric("first_full", 15_000);
        report.set_phase(
            "render_plan",
            PhaseReport {
                total_ns: 100,
                count: 1,
                nested: false,
            },
        );
        report.set_phase(
            "effect_build",
            PhaseReport {
                total_ns: 50,
                count: 2,
                nested: true,
            },
        );
        report.set_digest("output", "0123456789abcdef");

        let parsed = Report::parse(&report.render()).expect("round trip");
        assert_eq!(parsed, report);
        assert_eq!(parsed.metrics[PRIMARY], 15_000);
        assert_eq!(parsed.metrics["first_full"], 15_000);
        assert_eq!(parsed.metrics[REPEATED_FULL], 243);
        assert!(!parsed.phases["render_plan"].nested);
        assert!(parsed.phases["effect_build"].nested);
        assert!(
            !parsed.phases.contains_key("mask_prepare"),
            "an absent phase is not a measured zero"
        );
    }

    #[test]
    fn cold_profile_reports_backend_fallback() {
        let mut cohort = cohort();
        cohort.requested_backend = "gpu".to_string();
        cohort.executed_backend = "cpu".to_string();
        cohort.fallback = Some("unavailable".to_string());
        let mut report = Report::new(cohort);
        for name in REQUIRED {
            report.set_metric(name, 1);
        }
        let parsed = Report::parse(&report.render()).expect("round trip");
        assert_eq!(parsed.cohort.requested_backend, "gpu");
        assert_eq!(parsed.cohort.executed_backend, "cpu");
        assert_eq!(parsed.cohort.fallback.as_deref(), Some("unavailable"));
    }

    #[test]
    fn cold_profile_comparison_classifies_improvement_and_regression() {
        let baseline = reports(&[(1000, 500, 200), (1000, 500, 200), (1000, 500, 200)]);
        let improved = reports(&[(100, 500, 200), (100, 500, 200), (100, 500, 200)]);
        let comparison = compare(&baseline, &improved);
        assert_eq!(comparison.outcome, Outcome::Improved);
        assert_eq!(comparison.outcome.exit_code(), 0);

        let regressed = reports(&[(1000, 600, 200), (1000, 600, 200), (1000, 600, 200)]);
        let comparison = compare(&baseline, &regressed);
        assert_eq!(comparison.outcome, Outcome::Regressed);
        assert_eq!(comparison.outcome.exit_code(), 2);

        let control_regressed = reports(&[(100, 700, 200), (100, 700, 200), (100, 700, 200)]);
        let comparison = compare(&baseline, &control_regressed);
        assert_eq!(comparison.outcome, Outcome::Regressed);
    }

    #[test]
    fn cold_profile_comparison_rejects_invalid_and_insufficient() {
        let baseline = reports(&[(1000, 500, 200), (1000, 500, 200)]);
        let candidate = reports(&[(1000, 500, 200), (1000, 500, 200)]);
        let comparison = compare(&baseline, &candidate);
        assert_eq!(comparison.outcome, Outcome::Insufficient);
        assert_eq!(comparison.outcome.exit_code(), 4);

        let baseline = reports(&[(1000, 500, 200); 3]);
        let mut mismatched = reports(&[(100, 500, 200); 3]);
        mismatched[1].cohort.machine = "other".to_string();
        let comparison = compare(&baseline, &mismatched);
        assert_eq!(comparison.outcome, Outcome::Invalid);
        assert_eq!(comparison.outcome.exit_code(), 5);

        let mut missing = reports(&[(100, 500, 200); 3]);
        missing[2].metrics.remove(REPEATED_FULL);
        let comparison = compare(&baseline, &missing);
        assert_eq!(comparison.outcome, Outcome::Invalid);

        assert!(Report::parse("protocol=x\nversion=1\n").is_err());
        let mut text = report(1, 1, 1).render();
        text.push_str("metric.primary=1.5\n");
        assert!(Report::parse(&text).is_err());
        let mut text = report(1, 1, 1).render();
        text.push_str("metric.primary=NaN\n");
        assert!(Report::parse(&text).is_err());
    }

    #[test]
    fn cold_profile_comparison_marks_noise_inconclusive() {
        let baseline = reports(&[(100, 500, 200), (200, 500, 200), (300, 500, 200)]);
        let candidate = reports(&[(100, 500, 200), (200, 500, 200), (300, 500, 200)]);
        let comparison = compare(&baseline, &candidate);
        assert_eq!(comparison.outcome, Outcome::Inconclusive);
        assert_eq!(comparison.outcome.exit_code(), 3);
        assert!(comparison.metrics[0].noisy);
        assert_eq!(comparison.metrics[0].baseline.len(), 3, "raw samples kept");

        let baseline = reports(&[(1000, 500, 200), (1000, 500, 200), (1100, 500, 200)]);
        let candidate = reports(&[(1000, 500, 200), (1000, 500, 200), (1100, 500, 200)]);
        let comparison = compare(&baseline, &candidate);
        assert_eq!(
            comparison.outcome,
            Outcome::Inconclusive,
            "exactly at 10% is not noisy"
        );
        assert!(!comparison.metrics[0].noisy);

        let zero = reports(&[(0, 500, 200), (0, 500, 200), (10, 500, 200)]);
        let comparison = compare(&zero, &zero);
        assert!(comparison.metrics[0].noisy);
        assert_eq!(comparison.outcome, Outcome::Inconclusive);
    }

    #[test]
    fn cold_profile_comparison_exit_codes() {
        assert_eq!(Outcome::Improved.exit_code(), 0);
        assert_eq!(Outcome::Regressed.exit_code(), 2);
        assert_eq!(Outcome::Inconclusive.exit_code(), 3);
        assert_eq!(Outcome::Insufficient.exit_code(), 4);
        assert_eq!(Outcome::Invalid.exit_code(), 5);
    }
}
