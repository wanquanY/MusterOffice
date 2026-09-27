use crate::{Failure, FailureCode};

// Count serialization without retaining another copy of the input document.
pub(crate) fn check_size(
    value: &impl serde::Serialize,
    limit: usize,
    label: &'static str,
    check: &dyn Fn() -> bool,
) -> Result<(), Failure> {
    struct Count<'a> {
        bytes: usize,
        next_check: usize,
        limit: usize,
        label: &'static str,
        failure: Option<Failure>,
        check: &'a dyn Fn() -> bool,
    }
    impl std::io::Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.bytes >= self.next_check {
                if (self.check)() {
                    self.failure = Some(Failure::new(
                        FailureCode::Cancelled,
                        "computation cancelled",
                    ));
                    return Err(std::io::Error::other("cancelled"));
                }
                self.next_check = self.bytes.saturating_add(65536);
            }
            let Some(length) = self
                .bytes
                .checked_add(bytes.len())
                .filter(|n| *n <= self.limit)
            else {
                self.failure = Some(Failure::new(FailureCode::LimitExceeded, self.label));
                return Err(std::io::Error::other(self.label));
            };
            self.bytes = length;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count {
        bytes: 0,
        next_check: 0,
        limit,
        label,
        failure: None,
        check,
    };
    serde_json::to_writer(&mut count, value).map_err(|_| {
        count.failure.unwrap_or_else(|| {
            Failure::new(FailureCode::InputInvalid, "computation input serialization")
        })
    })?;
    if check() {
        return Err(Failure::new(
            FailureCode::Cancelled,
            "computation cancelled",
        ));
    }
    Ok(())
}
