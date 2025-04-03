use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

pub fn init_tracing(log_level: &str) {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::new(log_level))
        .init();
}

pub fn to_internal_error<E: std::fmt::Debug>(context: &str, e: E) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(format!("{} error: {:?}", context, e))
}
