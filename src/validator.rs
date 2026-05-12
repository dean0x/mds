use crate::ast::*;
use crate::error::MdsError;
use crate::scope::Scope;

/// Validate semantic correctness of a module AST.
/// Checks variable references, function arity, and type constraints
/// before evaluation. Block-scoped variables (e.g., @for loop vars)
/// are verified at evaluation time.
pub fn validate(nodes: &[Node], scope: &Scope) -> Result<(), MdsError> {
    for node in nodes {
        validate_node(node, scope)?;
    }
    Ok(())
}

fn validate_node(node: &Node, scope: &Scope) -> Result<(), MdsError> {
    match node {
        Node::Text(_) | Node::EscapedBrace => Ok(()),
        Node::Interpolation(interp) => validate_expr(&interp.expr, scope),
        Node::If(block) => {
            // Condition must be a defined variable (truthiness is checked at evaluation time)
            scope
                .get_var(&block.condition)
                .ok_or_else(|| MdsError::undefined_var(&block.condition))?;
            for node in &block.then_body {
                validate_node(node, scope)?;
            }
            if let Some(else_body) = &block.else_body {
                for node in else_body {
                    validate_node(node, scope)?;
                }
            }
            Ok(())
        }
        Node::For(block) => {
            // Iterable must be defined; loop var is block-scoped and checked at evaluation time.
            scope
                .get_var(&block.iterable)
                .ok_or_else(|| MdsError::undefined_var(&block.iterable))?;
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
            scope
                .get_namespace(&inc.alias)
                .ok_or_else(|| MdsError::undefined_var(&inc.alias))?;
            Ok(())
        }
    }
}

fn validate_expr(expr: &Expr, scope: &Scope) -> Result<(), MdsError> {
    match expr {
        Expr::Var(name) => {
            scope
                .get_var(name)
                .ok_or_else(|| MdsError::undefined_var(name))?;
            Ok(())
        }
        Expr::Call { name, args } => {
            let func = scope
                .get_function(name)
                .ok_or_else(|| MdsError::undefined_fn(name))?;
            if args.len() != func.params.len() {
                return Err(MdsError::arity(name, func.params.len(), args.len()));
            }
            validate_var_args(args, scope)
        }
        Expr::QualifiedCall {
            namespace,
            name,
            args,
        } => {
            let ns = scope
                .get_namespace(namespace)
                .ok_or_else(|| MdsError::undefined_var(namespace))?;
            let qualified = format!("{namespace}.{name}");
            let func = ns
                .functions
                .get(name)
                .ok_or_else(|| MdsError::undefined_fn(&qualified))?;
            if args.len() != func.params.len() {
                return Err(MdsError::arity(&qualified, func.params.len(), args.len()));
            }
            validate_var_args(args, scope)
        }
    }
}

/// Check that all variable arguments reference defined variables.
fn validate_var_args(args: &[Arg], scope: &Scope) -> Result<(), MdsError> {
    for arg in args {
        if let Arg::Var(var_name) = arg {
            if scope.get_var(var_name).is_none() {
                return Err(MdsError::undefined_var(var_name));
            }
        }
    }
    Ok(())
}
