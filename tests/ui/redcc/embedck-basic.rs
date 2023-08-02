// run-pass
// check-run-results

#![feature(rustc_attrs)]
#![feature(lang_items)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct Direct(RRef<i32>);
struct TwoRRefs(RRef<i32>, RRef<i32>);

// Detect trivial embedding and non-embedding assignments
fn main() {
    let r = RRef(4);
    let mut rc = RRef(Direct(RRef(1)));
    rc.0.0 = r;

    let r = RRef(4);
    let mut c = Direct(RRef(1));
    c.0 = r;

    let mut r = TwoRRefs(RRef(1), RRef(2));
    r.1 = RRef(3);
}

// FIXME: Check type in `compiler/rustc_middle/src/ty/context.rs`
#[lang = "redcc_print_embed"]
fn print_embed() {
    eprintln!("embed");
}

// more test cases:
    // embedding with different kinds of rvalues
        // enums too
    // embedding with different kinds of places
        // array
        // tuple
        // ref/pointer
        // enum
    // rvalue with embedded rref works
