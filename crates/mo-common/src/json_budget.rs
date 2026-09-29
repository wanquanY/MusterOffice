//! Bound serialization work without allocating a second serialized document.
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum JsonBudgetError {
    #[error("JSON serialization exceeds the byte budget")]
    Limit,
    #[error("JSON serialization was cancelled")]
    Cancelled,
    #[error("value could not be serialized as JSON")]
    Serialization,
}

pub fn check_json_size(
    value: &impl Serialize,
    limit: usize,
    check: &dyn Fn() -> bool,
) -> Result<usize, JsonBudgetError> {
    struct Count<'a> {
        bytes: usize,
        next_check: usize,
        limit: usize,
        failure: Option<JsonBudgetError>,
        check: &'a dyn Fn() -> bool,
    }
    impl std::io::Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.bytes >= self.next_check {
                if (self.check)() {
                    self.failure = Some(JsonBudgetError::Cancelled);
                    return Err(std::io::Error::other("cancelled"));
                }
                self.next_check = self.bytes.saturating_add(65_536);
            }
            let Some(length) = self
                .bytes
                .checked_add(bytes.len())
                .filter(|n| *n <= self.limit)
            else {
                self.failure = Some(JsonBudgetError::Limit);
                return Err(std::io::Error::other("JSON byte budget"));
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
        failure: None,
        check,
    };
    serde_json::to_writer(&mut count, value).map_err(|_| {
        count
            .failure
            .take()
            .unwrap_or(JsonBudgetError::Serialization)
    })?;
    if check() {
        return Err(JsonBudgetError::Cancelled);
    }
    Ok(count.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_escaped_utf8_and_checks_exact_budget_and_cancellation() {
        let input = "中\"\n😀";
        let exact = serde_json::to_vec(input).unwrap().len();
        assert_eq!(check_json_size(&input, exact, &|| false).unwrap(), exact);
        assert!(matches!(
            check_json_size(&input, exact - 1, &|| false),
            Err(JsonBudgetError::Limit)
        ));
        assert!(matches!(
            check_json_size(&input, exact, &|| true),
            Err(JsonBudgetError::Cancelled)
        ));
    }
}
