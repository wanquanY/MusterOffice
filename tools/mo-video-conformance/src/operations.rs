use mo_common::Ticks;
use mo_video_edit::{EditContract, Operation};
use mo_video_operations::*;
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    create: CreateProject,
    initial: Vec<Operation>,
    edit: Vec<Operation>,
    query: ListQuery,
}
fn run(input: &str) -> Result<Value> {
    let f: Fixture = decode(input, 1 << 20, &|| false)?;
    let limits = Limits::default();
    let empty = VideoProject::create(f.create, limits, &|| false)?;
    let initial = empty.prepare(
        &Transaction {
            contract: EditContract::V1,
            expected_digest: empty.digest().clone(),
            operations: f.initial,
        },
        &|| false,
    )?;
    let project = VideoProject::open(initial.snapshot.clone(), limits, &|| false)?;
    let mut cursor = None;
    let mut pages = vec![];
    loop {
        let page = project.list(
            PageRequest {
                query: f.query.clone(),
                cursor,
                limit: 1,
            },
            &|| false,
        )?;
        cursor = page.next.clone();
        pages.push(page);
        if cursor.is_none() {
            break;
        }
    }
    let edit = project.prepare(
        &Transaction {
            contract: EditContract::V1,
            expected_digest: project.digest().clone(),
            operations: f.edit,
        },
        &|| false,
    )?;
    let changed = VideoProject::open(edit.snapshot.clone(), limits, &|| false)?;
    let inverse = changed.prepare_inverse(
        &InverseRequest {
            expected_digest: changed.digest().clone(),
            inverse: edit.inverse.clone(),
        },
        &|| false,
    )?;
    let invalid = project
        .frame(
            FrameRequest {
                expected_digest: project.digest().clone(),
                frame: Ticks::new(-1),
            },
            &|| false,
        )
        .unwrap_err();
    Ok(
        json!({"initial":initial,"empty":empty.summary(&||false)?,"summary":project.summary(&||false)?,"pages":pages,"edit":edit,"inverse":inverse,"resources":project.resources(&||false)?,"frame":project.frame(FrameRequest {expected_digest:project.digest().clone(),frame:Ticks::new(1)},&||false)?,"invalid":invalid}),
    )
}
pub fn video_operations_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"error":error}).to_string(),
    }
}
