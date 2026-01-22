use std::time::Instant;

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub trace_id: String,
    pub start_time: Instant,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        }
    }

    pub fn with_trace_id(trace_id: String) -> Self {
        Self {
            trace_id,
            start_time: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_generates_uuid() {
        let ctx = RequestContext::new();
        // UUID v4 format: 8-4-4-4-12 hex chars
        assert_eq!(ctx.trace_id.len(), 36);
        assert!(ctx.trace_id.chars().filter(|c| *c == '-').count() == 4);
    }

    #[test]
    fn test_new_generates_unique_ids() {
        let ctx1 = RequestContext::new();
        let ctx2 = RequestContext::new();
        assert_ne!(ctx1.trace_id, ctx2.trace_id);
    }

    #[test]
    fn test_with_trace_id() {
        let custom_id = "custom-trace-123".to_string();
        let ctx = RequestContext::with_trace_id(custom_id.clone());
        assert_eq!(ctx.trace_id, custom_id);
    }

    #[test]
    fn test_elapsed_increases() {
        let ctx = RequestContext::new();
        let elapsed1 = ctx.elapsed();
        thread::sleep(Duration::from_millis(10));
        let elapsed2 = ctx.elapsed();
        assert!(elapsed2 > elapsed1);
    }

    #[test]
    fn test_default_is_new() {
        let ctx = RequestContext::default();
        assert_eq!(ctx.trace_id.len(), 36);
    }

    #[test]
    fn test_clone() {
        let ctx1 = RequestContext::new();
        let ctx2 = ctx1.clone();
        assert_eq!(ctx1.trace_id, ctx2.trace_id);
    }

    #[test]
    fn test_debug() {
        let ctx = RequestContext::with_trace_id("test-id".to_string());
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("test-id"));
    }
}
