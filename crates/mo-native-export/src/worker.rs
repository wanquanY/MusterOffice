use crate::{storage::*, wire::*, *};
use mo_presentation_delivery::{DeliveryLimits, PreviewRenderer};
use mo_presentation_operations::compute_export;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

/// Runs exactly one computation. The parent owns authorization, cancellation,
/// process termination, spool recovery and the final atomic product commit.
/// `spool_root` and `renderer` are trusted operator capabilities, never JSON.
pub fn run_worker(
    input: &mut impl Read,
    output: &mut impl Write,
    spool_root: &Path,
    renderer: &mut dyn PreviewRenderer,
) -> Result<(), Failure> {
    let _lease = mo_native_io::ExecutionSpoolLease::join(spool_root).map_err(storage_failure)?;
    let request: Request = read(input)?;
    request.validate(&renderer.identity())?;
    let mut assets = Inputs(BTreeMap::new());
    for asset in &request.assets {
        let reader = receive(input, spool_root, asset.descriptor.byte_length.get())?;
        assets.0.insert(asset.id.clone(), (asset.clone(), reader));
    }
    let mut trailing = [0; 1];
    if input.read(&mut trailing).map_err(storage_failure)? != 0 {
        return Err(invalid("trailing export input bytes"));
    }
    // All bounded input has arrived before returning a computational failure.
    // This preserves the transport's send-then-receive backpressure contract.
    let candidate = (|| {
        for (asset, reader) in assets.0.values() {
            verify(
                reader,
                asset.descriptor.byte_length.get(),
                &asset.descriptor.sha256,
            )?;
        }
        compute_export(
            &request.request.computation(),
            request.snapshot,
            &assets,
            &mut Outputs {
                root: spool_root.to_path_buf(),
            },
            renderer,
            DeliveryLimits::default(),
            &|| false,
        )
    })();
    match candidate {
        Ok(candidate) => {
            let receipt = candidate.receipt();
            output_preflight(&receipt.bundle.assets)?;
            let artifacts = candidate.delivery().artifacts();
            if artifacts.len() != receipt.bundle.assets.len()
                || artifacts.iter().zip(&receipt.bundle.assets).any(|(a, b)| {
                    let a = a.asset();
                    a.id != b.id
                        || a.sha256 != b.sha256
                        || a.byte_length != b.byte_length
                        || a.media_type != b.media_type
                        || a.role != b.role
                })
            {
                return Err(invalid("candidate output ordering or identity"));
            }
            write(
                output,
                &Response::Prepared {
                    request_digest: candidate.request_digest().clone(),
                    receipt: Box::new(receipt),
                },
            )?;
            for artifact in artifacts {
                stream(
                    artifact.reader(),
                    artifact.asset().byte_length.get(),
                    output,
                )?;
            }
        }
        Err(error) => write(output, &Response::Failed { error })?,
    }
    output.flush().map_err(storage_failure)
}
