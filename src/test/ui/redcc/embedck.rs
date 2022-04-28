// build-pass

#![feature(rustc_attrs)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct ContainsRRefDirect(RRef<i32>);

// Detect trivial embedding and non-embedding assignments
fn main() {
    let r = RRef(4);
    let mut rc = RRef(ContainsRRefDirect(RRef(1)));
    rc.0.0 = r;

    let r = RRef(4);
    let mut c = ContainsRRefDirect(RRef(1));
    c.0 = r;
}

// more test cases:
    // Indirect embedding
    // Sibling RRef does not trigger embed
    // embedding with different kinds of rvalues
        // enums too
    // embedding with different kinds of places
        // array
        // tuple
        // ref/pointer
        // enum
    // rvalue with embedded rref works
