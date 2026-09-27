//! One stateless export per process. The parent retains the only task and
//! commit authority, including termination and execution-directory cleanup.
mod renderer;
use std::{io, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [flag, root, lease] = args.as_slice() else {
        return Err(
            "expected --spool-dir <private execution directory> --execution-lease-v1".into(),
        );
    };
    if flag != "--spool-dir" || lease != "--execution-lease-v1" {
        return Err("expected --spool-dir and --execution-lease-v1".into());
    }
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("spool directory missing".into());
    }
    let identity = mo_presentation_delivery::RendererIdentity {
        implementation_sha256: mo_native_export::executable_digest(&std::env::current_exe()?)?,
        profile: mo_presentation_compile::source_resource_page::PROFILE.into(),
    };
    mo_native_export::run_worker(
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &root,
        &mut renderer::Renderer { identity },
    )?;
    Ok(())
}
