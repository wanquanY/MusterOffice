//! Shared single-plan lifecycle for author and imported-source samplers.
use crate::{PlaybackBinding, PlaybackSessionFailureCode as Code};
pub(crate) struct Failure {
    pub code: Code,
    pub message: &'static str,
}
pub(crate) trait Bound {
    fn binding(&self) -> &PlaybackBinding;
}
#[derive(Default)]
pub(crate) enum Owner<T> {
    #[default]
    Empty,
    Ready(Box<T>),
    Disposed(PlaybackBinding),
}
impl<T: Bound> Owner<T> {
    pub fn vacant(&self) -> Result<(), Failure> {
        match self {
            Self::Empty => Ok(()),
            Self::Ready(_) => Err(Failure {
                code: Code::AlreadyPrepared,
                message: "dispose and use a new owner for a different plan",
            }),
            Self::Disposed(_) => Err(Failure {
                code: Code::Disposed,
                message: "session disposed",
            }),
        }
    }
    pub fn ready(&mut self, binding: &PlaybackBinding) -> Result<&mut T, Failure> {
        match self {
            Self::Ready(r) => {
                if r.binding() == binding {
                    Ok(r)
                } else {
                    Err(Failure {
                        code: Code::BindingConflict,
                        message: "session/revision/generation mismatch",
                    })
                }
            }
            Self::Empty => Err(Failure {
                code: Code::NotPrepared,
                message: "session has no plan",
            }),
            Self::Disposed(_) => Err(Failure {
                code: Code::Disposed,
                message: "session disposed",
            }),
        }
    }
    pub fn dispose(
        &mut self,
        binding: PlaybackBinding,
        check: &dyn Fn() -> bool,
    ) -> Result<(), Failure> {
        if let Self::Disposed(previous) = self {
            if previous != &binding {
                return Err(Failure {
                    code: Code::BindingConflict,
                    message: "disposed binding mismatch",
                });
            }
        } else {
            self.ready(&binding)?;
            if check() {
                return Err(Failure {
                    code: Code::Cancelled,
                    message: "playback session cancelled",
                });
            }
            *self = Self::Disposed(binding);
        }
        Ok(())
    }
}
