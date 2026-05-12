use crate::ast::*;
use crate::error::MdsError;
use crate::scope::Scope;

/// Validate semantic correctness of a module AST.
/// Checks variable references, function arity, and type constraints
/// before evaluation. Block-scoped variables (e.g., @for loop vars)
/// are verified at evaluation time.
pub fn validate(nodes: &[Node], scope: &Scope, file: &str, source: &str) -> Result<(), MdsError> {
    for node in nodes {
        validate_node(node, scope, file, source)?;
    }
    Ok(())
}

fn validate_node(node: &Node, scope: &Scope, file: &str, source: &str) -> Result<(), MdsError> {
    match node {
        Node::Text(_) | Node::EscapedBrace => Ok(()),
        Node::Interpolation(interp) => {
            validate_expr(&interp.expr, scope, file, source, interp.offset, interp.len)
        }
        Node::If(block) => {
            // Condition must be a defined variable (truthiness is checked at evaluation time)
            scope.get_var(&block.condition).ok_or_else(|| {
                MdsError::undefined_var_at(
                    &block.condition,
                    file,
                    source,
                    block.offset,
                    block.condition.len(),
                )
            })?;
            for node in &block.then_body {
                validate_node(node, scope, file, source)?;
            }
            if let Some(else_body) = &block.else_body {
                for node in else_body {
                    validate_node(node, scope, file, source)?;
                }
            }
            Ok(())
        }
        Node::For(block) => {
            scope.get_var(&block.iterable).ok_or_else(|| {
                MdsError::undefined_var_at(
                    &block.iterable,
                    file,
                    source,
                    block.offset,
                    block.iterable.len(),
                )
            })?;
            let mut inner = scope.clone();
            inner.set_var(&block.var, crate::value::Value::Null);
            for node in &block.body {
                validate_node(node, &inner, file, source)?;
            }
            Ok(())
        }
        Node::Define(_) => {
            // Function bodies are validated when called
            Ok(())
        }
        Node::Import(_) | Node::Export(_) => {
            // Handled by resolver
            Ok(())
        }
        Node::Include(inc) => {
            // Verify the referenced namespace exists (must have been @import-ed)
            scope.get_namespace(&inc.alias).ok_or_else(|| {
                MdsError::undefined_var_at(&inc.alias, file, source, inc.offset, inc.alias.len())
            })?;
            Ok(())
        }
    }
}

fn validate_expr(
    expr: &Expr,
    scope: &Scope,
    file: &str,
    source: &str,
    offset: usize,
    len: usize,
) -> Result<(), MdsError> {
    match expr {
        Expr::Var(name) => {
            scope.get_var(name).ok_or_else(|| {
                MdsError::undefined_var_at(name, file, source, offset, name.len())
            })?;
            Ok(())
        }
        Expr::Call { name, args } => {
            let func = scope
                .get_function(name)
                .ok_or_else(|| MdsError::undefined_fn_at(name, file, source, offset, len))?;
            if args.len() != func.params.len() {
                return Err(MdsError::ArityMismatch {
                    name: name.clone(),
                    expected: func.params.len(),
                    got: args.len(),
                    span: Some(miette::SourceSpan::new(offset.into(), len)),
                    src: Some(std::sync::Arc::new(miette::NamedSource::new(
                        file,
                        source.to_string(),
                    ))),
                });
            }
            validate_var_args(args, scope, file, source, offset)
        }
        Expr::QualifiedCall {
            namespace,
            name,
            args,
        } => {
            let ns = scope
                .get_namespace(namespace)
                .ok_or_else(|| MdsError::undefined_var_at(namespace, file, source, offset, len))?;
            let qualified = format!("{namespace}.{name}");
            let func = ns
                .functions
                .get(name)
                .ok_or_else(|| MdsError::undefined_fn_at(&qualified, file, source, offset, len))?;
            if args.len() != func.params.len() {
                return Err(MdsError::ArityMismatch {
                    name: qualified,
                    expected: func.params.len(),
                    got: args.len(),
                    span: Some(miette::SourceSpan::new(offset.into(), len)),
                    src: Some(std::sync::Arc::new(miette::NamedSource::new(
                        file,
                        source.to_string(),
                    ))),
                });
            }
            validate_var_args(args, scope, file, source, offset)
        }
    }
}

/// Check that all variable arguments reference defined variables.
fn validate_var_args(
    args: &[Arg],
    scope: &Scope,
    file: &str,
    source: &str,
    offset: usize,
) -> Result<(), MdsError> {
    for arg in args {
        if let Arg::Var(var_name) = arg {
            if scope.get_var(var_name).is_none() {
                return Err(MdsError::undefined_var_at(
                    var_name,
                    file,
                    source,
                    offset,
                    var_name.len(),
                ));
            }
        }
    }
    Ok(())
}
