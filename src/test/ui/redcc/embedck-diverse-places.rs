// build-pass

#![feature(rustc_attrs)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct Direct(RRef<i32>);

fn main() {
    // place is array
    // let mut array = RRef([RRef(0), RRef(1)]);
    // array.0[0] = RRef(2);

    // let mut array = [RRef(0), RRef(1)];
    // array[0] = RRef(2);
    // FIXME: turns out arrays are more complicated than i thought

    // tuple
    let mut tuple = RRef((RRef(0), RRef(1)));
    tuple.0.0 = RRef(2);

    let mut tuple = (RRef(0), RRef(1));
    tuple.0 = RRef(2);

    // reference
    let mut r = &mut RRef(RRef(1));
    r.0 = RRef(0);

    let mut r = &mut Direct(RRef(1));
    r.0 = RRef(0);
}
