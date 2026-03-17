// apcore-cli — JSON Schema $ref inliner.
// Protocol spec: FE-08 (resolve_refs)

use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced during `$ref` resolution.
#[derive(Debug, Error)]
pub enum RefResolverError {
    /// A `$ref` target could not be found in the schema's `$defs`.
    #[error("unresolvable $ref '{reference}' in module '{module_id}' (exit 45)")]
    Unresolvable {
        reference: String,
        module_id: String,
    },

    /// A circular reference chain was detected (exit 48).
    #[error("circular $ref detected in module '{module_id}' (exit 48)")]
    Circular { module_id: String },

    /// The maximum recursion depth was exceeded.
    #[error("$ref resolution exceeded max depth {max_depth} in module '{module_id}'")]
    MaxDepthExceeded { max_depth: usize, module_id: String },
}

// ---------------------------------------------------------------------------
// resolve_refs
// ---------------------------------------------------------------------------

/// Inline all `$ref` pointers in a JSON Schema value.
///
/// Resolves `$ref` values by looking them up in `schema["$defs"]` and
/// substituting the referenced schema in-place. Handles nested schemas
/// recursively up to `max_depth`.
///
/// # Arguments
/// * `schema`    — mutable JSON Schema value (modified in place, also returned)
/// * `max_depth` — maximum recursion depth before raising `MaxDepthExceeded`
/// * `module_id` — module identifier for error messages
///
/// # Errors
/// * `RefResolverError::Unresolvable` — unknown `$ref` target (exit 45)
/// * `RefResolverError::Circular`     — circular reference (exit 48)
/// * `RefResolverError::MaxDepthExceeded` — depth limit reached
pub fn resolve_refs(
    schema: &mut Value,
    max_depth: usize,
    module_id: &str,
) -> Result<Value, RefResolverError> {
    // TODO: walk the schema tree, detect $ref strings, look up in $defs,
    //       track visited refs to detect cycles, enforce max_depth.
    let _ = (schema, max_depth, module_id);
    todo!("resolve_refs")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_refs_no_refs_unchanged() {
        // A schema without any $ref must be returned unchanged.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_refs_simple_ref() {
        // A single $ref must be inlined from $defs.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_refs_unresolvable_returns_error() {
        // An unknown $ref must yield RefResolverError::Unresolvable.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_refs_circular_returns_error() {
        // A circular $ref chain must yield RefResolverError::Circular.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_refs_max_depth_exceeded() {
        // Exceeding max_depth must yield RefResolverError::MaxDepthExceeded.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_refs_nested_defs() {
        // $refs in nested properties must all be resolved.
        assert!(false, "not implemented");
    }
}
