//! Snapshot-bound deterministic timeline sampling. Hosts own clock, events and CAS.
use mo_common::{RationalTime, SlideId, from_json_str};
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use mo_presentation_model::ValidationLimits;
pub use mo_timeline::{EvaluatedFrame, EventHistory, PlaybackBinding, TimelineLimits};
use mo_timeline::{TimelineError, TimelinePlan};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineEvaluateRequest {
    pub snapshot: SnapshotRecord,
    pub slide: SlideId,
    pub binding: PlaybackBinding,
    pub at: RationalTime,
    pub history: Option<EventHistory>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineFailureCode {
    InputInvalid,
    RevisionConflict,
    EventHistoryRequired,
    EventHistoryInvalid,
    LimitExceeded,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineFailure {
    pub code: TimelineFailureCode,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum TimelineEvaluateResponse {
    Evaluated { frame: Box<EvaluatedFrame> },
    Error { error: TimelineFailure },
}
fn fail(code: TimelineFailureCode, message: impl ToString) -> TimelineFailure {
    TimelineFailure {
        code,
        message: message.to_string(),
    }
}
pub(crate) fn failure(error: TimelineError) -> TimelineFailure {
    let code = match &error {
        TimelineError::Cancelled => TimelineFailureCode::Cancelled,
        TimelineError::Limit(_) => TimelineFailureCode::LimitExceeded,
        TimelineError::MissingEventHistory => TimelineFailureCode::EventHistoryRequired,
        TimelineError::EventHistory(_) => TimelineFailureCode::EventHistoryInvalid,
        _ => TimelineFailureCode::InputInvalid,
    };
    fail(code, error)
}
pub(crate) fn restore_snapshot(
    record: SnapshotRecord,
    binding: &PlaybackBinding,
    check: &dyn Fn() -> bool,
) -> Result<Snapshot, TimelineFailure> {
    if check() {
        return Err(failure(TimelineError::Cancelled));
    }
    if binding.revision != record.revision {
        return Err(fail(
            TimelineFailureCode::RevisionConflict,
            "playback revision does not match snapshot",
        ));
    }
    let snapshot = Snapshot::restore(record, ValidationLimits::default())
            .map_err(|e| {
                let over_budget = matches!(&e, mo_presentation_edit::EditError::InvalidDocument(report)
                    if report.issues.iter().any(|issue| issue.code == mo_presentation_model::ValidationCode::LimitExceeded));
                fail(if over_budget { TimelineFailureCode::LimitExceeded } else { TimelineFailureCode::InputInvalid }, e)
            })?;
    if check() {
        return Err(failure(TimelineError::Cancelled));
    }
    Ok(snapshot)
}
pub fn evaluate_timeline(
    request: TimelineEvaluateRequest,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> TimelineEvaluateResponse {
    let result = (|| {
        let snapshot = restore_snapshot(request.snapshot, &request.binding, check)?;
        if !snapshot.document().slides.contains_key(&request.slide) {
            return Err(fail(
                TimelineFailureCode::InputInvalid,
                "slide does not exist",
            ));
        }
        let empty = mo_timeline::Timeline {
            format: mo_timeline::TimelineVersion::V01,
            tree: None,
            nodes: vec![],
        };
        let timeline = snapshot
            .document()
            .timelines
            .get(&request.slide)
            .unwrap_or(&empty);
        let plan = TimelinePlan::compile(timeline, limits, check).map_err(failure)?;
        plan.evaluate(
            &request.binding,
            request.at,
            request.history.as_ref(),
            check,
        )
        .map_err(failure)
    })();
    match result {
        Ok(frame) => TimelineEvaluateResponse::Evaluated {
            frame: Box::new(frame),
        },
        Err(error) => TimelineEvaluateResponse::Error { error },
    }
}
pub fn evaluate_timeline_json(input: &str, check: &dyn Fn() -> bool) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        TimelineEvaluateResponse::Error {
            error: fail(TimelineFailureCode::LimitExceeded, "timeline request bytes"),
        }
    } else {
        match from_json_str::<TimelineEvaluateRequest>(input) {
            Ok(request) => evaluate_timeline(request, TimelineLimits::default(), check),
            Err(e) => TimelineEvaluateResponse::Error {
                error: fail(TimelineFailureCode::InputInvalid, e),
            },
        }
    };
    serde_json::to_string(&response).expect("typed timeline response")
}
