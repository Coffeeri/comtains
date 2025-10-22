use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ExprArray, ExprLit, Lit, Result, Token,
};

/// Build a [`ByteSet`](::comtains::ByteSet) from compile-time byte sequences.
///
/// The macro accepts byte string literals (`b"..."`), UTF-8 string literals
/// (`"..."`, which are converted to bytes), or arrays of integer / byte
/// literals (`[0xAA, 0xBB]`). Duplicate entries are removed automatically.
///
/// # Example
/// ```rust,ignore
/// use comtains::{byte_set, ByteSet};
///
/// const METHODS: ByteSet = byte_set![
///     b"GET",
///     b"POST",
///     [b'P', b'U', b'T'],
/// ];
///
/// assert!(METHODS.contains(b"GET"));
/// assert!(METHODS.contains(b"PUT"));
/// assert!(!METHODS.contains(b"DELETE"));
/// ```
#[proc_macro]
pub fn byte_set(input: TokenStream) -> TokenStream {
    let sequences = parse_macro_input!(input as SequenceList);
    match expand_byte_set(sequences.0) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct SequenceList(Vec<Vec<u8>>);

impl Parse for SequenceList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut sequences = Vec::new();

        while !input.is_empty() {
            let expr: Expr = input.parse()?;
            sequences.push(parse_sequence(expr)?);

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else {
                break;
            }
        }

        Ok(Self(sequences))
    }
}

fn parse_sequence(expr: Expr) -> Result<Vec<u8>> {
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::ByteStr(bytes) => Ok(bytes.value()),
            Lit::Str(s) => Ok(s.value().into_bytes()),
            _ => Err(syn::Error::new(
                Span::call_site(),
                "expected byte string (b\"..\") or string literal",
            )),
        },
        Expr::Array(ExprArray { elems, .. }) => {
            let mut out = Vec::with_capacity(elems.len());
            for elem in elems {
                out.push(parse_byte(elem)?);
            }
            Ok(out)
        }
        Expr::Reference(expr_ref) => parse_sequence(*expr_ref.expr),
        _ => Err(syn::Error::new(
            Span::call_site(),
            "each sequence must be a byte string literal, string literal, or array of byte literals",
        )),
    }
}

fn parse_byte(expr: Expr) -> Result<u8> {
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Int(int_lit) => {
                let value = int_lit.base10_parse::<u16>()?;
                if value > u8::MAX as u16 {
                    Err(syn::Error::new(
                        int_lit.span(),
                        "byte value must be between 0 and 255",
                    ))
                } else {
                    Ok(value as u8)
                }
            }
            Lit::Byte(byte_lit) => Ok(byte_lit.value()),
            Lit::Char(char_lit) => {
                let value = char_lit.value() as u32;
                if value > u8::MAX as u32 {
                    Err(syn::Error::new(
                        char_lit.span(),
                        "character literal must fit into a single byte",
                    ))
                } else {
                    Ok(value as u8)
                }
            }
            _ => Err(syn::Error::new(
                lit.span(),
                "expected integer, byte, or character literal inside array",
            )),
        },
        _ => Err(syn::Error::new(
            expr.span(),
            "array elements must be byte literals or integers",
        )),
    }
}

fn expand_byte_set(mut sequences: Vec<Vec<u8>>) -> Result<TokenStream2> {
    if sequences.is_empty() {
        return Ok(expand_empty_byte_set());
    }

    let mut unique = BTreeSet::new();
    sequences.retain(|seq| unique.insert(seq.clone()));

    let trie = build_trie(&sequences);
    let root_expr = generate_node_expr(&trie, 0, 0);

    let node_count = trie.len();
    let mut node_defs = Vec::with_capacity(node_count);
    let mut edge_defs = Vec::new();
    let mut edge_start = 0usize;

    for node in &trie {
        let terminal = node.terminal;
        let child_start = edge_start;
        let child_len = node.children.len();

        node_defs.push(quote! {
            ::comtains::debug::DebugNode {
                terminal: #terminal,
                child_start: #child_start,
                child_len: #child_len,
            }
        });

        for child in &node.children {
            let byte_lit = syn::LitInt::new(&format!("{}", child.byte), Span::call_site());
            let target = child.target;
            let weight = child.weight;
            edge_defs.push(quote! {
                ::comtains::debug::DebugEdge {
                    byte: #byte_lit,
                    target: #target,
                    weight: #weight,
                }
            });
        }

        edge_start += child_len;
    }

    let edge_count = edge_defs.len();
    let len_lit = sequences.len();

    let module_ident = unique_module_ident();

    Ok(quote! {{
        mod #module_ident {
            #[inline(always)]
            pub fn contains(candidate: &[u8]) -> bool {
                let len = candidate.len();
                #root_expr
            }

            pub const NODES: [::comtains::debug::DebugNode; #node_count] = [
                #( #node_defs ),*
            ];

            pub const EDGES: [::comtains::debug::DebugEdge; #edge_count] = [
                #( #edge_defs ),*
            ];

            pub const METADATA: ::comtains::debug::ByteSetMetadata =
                ::comtains::debug::ByteSetMetadata {
                    nodes: &NODES,
                    edges: &EDGES,
                };
        }

        ::comtains::ByteSet::__from_parts(
            #module_ident::contains,
            #len_lit,
            &#module_ident::METADATA,
        )
    }})
}

fn expand_empty_byte_set() -> TokenStream2 {
    let module_ident = unique_module_ident();
    quote! {{
        mod #module_ident {
            #[inline(always)]
            pub fn contains(_: &[u8]) -> bool {
                false
            }

            pub const NODES: [::comtains::debug::DebugNode; 1] = [
                ::comtains::debug::DebugNode {
                    terminal: false,
                    child_start: 0,
                    child_len: 0,
                }
            ];

            pub const EDGES: [::comtains::debug::DebugEdge; 0] = [];

            pub const METADATA: ::comtains::debug::ByteSetMetadata =
                ::comtains::debug::ByteSetMetadata {
                    nodes: &NODES,
                    edges: &EDGES,
                };
        }

        ::comtains::ByteSet::__from_parts(
            #module_ident::contains,
            0usize,
            &#module_ident::METADATA,
        )
    }}
}

#[derive(Clone)]
struct Node {
    terminal: bool,
    children: Vec<Edge>,
}

#[derive(Clone)]
struct Edge {
    byte: u8,
    target: usize,
    weight: usize,
}

fn build_trie(sequences: &[Vec<u8>]) -> Vec<Node> {
    let mut nodes = vec![Node {
        terminal: false,
        children: Vec::new(),
    }];

    for sequence in sequences {
        let mut node_idx = 0usize;
        if sequence.is_empty() {
            nodes[node_idx].terminal = true;
            continue;
        }

        for &byte in sequence {
            let mut next = None;
            for edge in nodes[node_idx].children.iter_mut() {
                if edge.byte == byte {
                    edge.weight += 1;
                    next = Some(edge.target);
                    break;
                }
            }

            if let Some(target) = next {
                node_idx = target;
            } else {
                let next_node = nodes.len();
                nodes.push(Node {
                    terminal: false,
                    children: Vec::new(),
                });
                nodes[node_idx].children.push(Edge {
                    byte,
                    target: next_node,
                    weight: 1,
                });
                node_idx = next_node;
            }
        }

        nodes[node_idx].terminal = true;
    }

    for node in &mut nodes {
        node.children
            .sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.byte.cmp(&b.byte)));
    }

    nodes
}

fn generate_node_expr(nodes: &[Node], index: usize, depth: usize) -> TokenStream2 {
    let node = &nodes[index];
    let depth_lit = syn::LitInt::new(&format!("{depth}usize"), Span::call_site());

    if node.children.is_empty() {
        if node.terminal {
            quote! { len == #depth_lit }
        } else {
            quote! { false }
        }
    } else {
        let terminal_expr = if node.terminal {
            quote! { true }
        } else {
            quote! { false }
        };

        let mut arms = Vec::with_capacity(node.children.len());
        for edge in &node.children {
            let byte_lit = syn::LitInt::new(&format!("{}", edge.byte), Span::call_site());
            let child_expr = generate_node_expr(nodes, edge.target, depth + 1);
            arms.push(quote! {
                Some(&#byte_lit) => #child_expr
            });
        }

        quote! {
            if len == #depth_lit {
                #terminal_expr
            } else {
                match candidate.get(#depth_lit) {
                    #( #arms, )*
                    _ => false,
                }
            }
        }
    }
}

fn unique_module_ident() -> proc_macro2::Ident {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format_ident!("__comtains_byte_set_{}", id)
}
