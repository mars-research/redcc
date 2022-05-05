// build-pass

#![feature(rustc_attrs)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct Direct(RRef<i32>);
struct Indirect(Direct);

// Detect trivial embedding and non-embedding indirect assignments
fn main() {
    let r = RRef(4);
    let mut rc = RRef(Indirect(Direct(RRef(1))));
    rc.0.0.0 = r;

    let r = RRef(4);
    let mut c = Indirect(Direct(RRef(1)));
    c.0.0 = r;
}
