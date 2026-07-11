#[cfg(test)]
mod tests {
    use sir_builder::Builder;
    use sir_types::{ConstantData, Type, Span};
    use crate::framework::run_benchmark;

    fn unknown_span() -> Span {
        Span::unknown()
    }
}
