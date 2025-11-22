use anyhow::Result;
use rand::Rng;
use rand::seq::IndexedRandom;
use swc_atoms::Atom;
use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith};
use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use crate::mutators::js_objects::js_objects::JsMethodSignature;
use crate::mutators::js_objects::js_types::JsObjectType;

pub fn build_ctor_expr(
    object_type: &str,
    args: Vec<Expr>,
) -> Expr {
    Expr::New(NewExpr {
        span: DUMMY_SP,
        callee: Box::new(Expr::Ident(Ident {
            span: DUMMY_SP,
            sym: Atom::from(object_type),
            optional: false,
            ctxt: SyntaxContext::empty(),
        })),
        args: Some(args.into_iter().map(|arg| ExprOrSpread {
            spread: None,
            expr: Box::new(arg),
        }).collect()),
        type_args: None,
        ctxt: SyntaxContext::empty(),
    })
}

pub fn build_var_decl(
    var_name: &str,
    init_expr: Expr,
) -> VarDecl {
    VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Let,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: Ident {
                    span: DUMMY_SP,
                    sym: Atom::from(var_name),
                    optional: false,
                    ctxt: SyntaxContext::empty(),
                },
                type_ann: None,
            }),
            init: Some(Box::new(init_expr)),
            definite: false,
        }],
    }
}

pub fn build_property_call(
    var_name: &str,
    property_name: &str,
    arg_exprs: Vec<Expr>,
) -> Expr {
    let callee_expr = Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(Ident {
            span: DUMMY_SP,
            sym: Atom::from(var_name),
            optional: false,
            ctxt: SyntaxContext::empty(),
        })),
        prop: MemberProp::Ident(IdentName::new(Atom::from(property_name), DUMMY_SP)),
    });
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(callee_expr)),
        args: arg_exprs
            .into_iter()
            .map(|arg| ExprOrSpread {
                spread: None,
                expr: Box::new(arg),
            })
            .collect(),
        type_args: None,
    })
}

pub fn build_args(
    sig: &JsMethodSignature,
    value_pool: &[String],
) -> Vec<Expr> {
    sig.types()
    .iter()
    .map(|ty| build_arg_expr(*ty, value_pool))
    .collect()
}

fn build_arg_expr(ty: JsObjectType, value_pool: &[String]) -> Expr {
    let mut rng = rand::rngs::ThreadRng::default();
    if !value_pool.is_empty() && rng.random_bool(0.35) {
        if let Some(existing) = value_pool.choose(&mut rng) {
            return Expr::Ident(Ident {
                span: DUMMY_SP,
                sym: Atom::from(existing.as_str()),
                optional: false,
                ctxt: SyntaxContext::empty(),
            });
        }
    }
    
    match ty {
        JsObjectType::Number => {
            // Bias toward interesting edge numbers.
            let specials = [0.0f64, -0.0, -1.0, 1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0xffff_ffffu32 as f64];
            let v = if rng.random_bool(0.15) {
                *specials.choose(&mut rng).unwrap_or(&0.0)
            } else {
                rng.random_range(-16i32..=128i32) as f64
            };
            Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: v,
                raw: None,
            }))
        }
        JsObjectType::Boolean => {
            let v = rng.random_bool(0.5);
            Expr::Lit(Lit::Bool(Bool {
                span: DUMMY_SP,
                value: v,
            }))
        }
        JsObjectType::JsString => {
            let choices = ["foo", "bar", "baz", "qux", "こんにちは"];
            let s = choices.choose(&mut rng).copied().unwrap_or("s");
            Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: Atom::from(s).into(),
                raw: None,
            }))
        }
        JsObjectType::Object => Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: Vec::new(),
        }),
        JsObjectType::Array => Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: Vec::new(),
        }),
        JsObjectType::Function => {
            // Prefer reusing a value from scope/value pool when asked for a function.
            if !value_pool.is_empty() && rng.random_bool(0.5) {
                if let Some(existing) = value_pool.choose(&mut rng) {
                    return Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: Atom::from(existing.as_str()),
                        optional: false,
                        ctxt: SyntaxContext::empty(),
                    });
                }
            }
            
            Expr::Fn(FnExpr {
                ident: None,
                function: Box::new(Function {
                    params: Vec::new(),
                    decorators: Vec::new(),
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    body: Some(BlockStmt {
                        span: DUMMY_SP,
                        ctxt: Default::default(),
                        stmts: Vec::new(),
                    }),
                    is_generator: false,
                    is_async: false,
                    type_params: None,
                    return_type: None,
                }),
            })
        }
        JsObjectType::Any => {
            let choice = rng.random_range(0..5);
            let concrete = match choice {
                0 => JsObjectType::Number,
                1 => JsObjectType::JsString,
                2 => JsObjectType::Object,
                3 => JsObjectType::Function,
                _ => JsObjectType::Array,
            };
            build_arg_expr(concrete, value_pool)
        }
    }
}
