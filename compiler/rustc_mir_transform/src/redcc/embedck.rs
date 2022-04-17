use rustc_middle::ty::{self, AdtDef, Ty, TyCtxt};
use rustc_span::symbol::sym;

pub fn contains_rref<'tcx>(tcx: TyCtxt<'tcx>, t: Ty<'tcx>) -> bool {
    match t.kind() {
        ty::Adt(adt, _) => {
            tcx.is_diagnostic_item(sym::RRef, adt.did()) || adt_fields_contain_rref(tcx, adt)
        }
        _ => false,
    }
}

// FIXME: this breaks for recursive types
fn adt_fields_contain_rref<'tcx>(tcx: TyCtxt<'tcx>, adt: &'tcx AdtDef<'tcx>) -> bool {
    adt.variants().iter().any(|v| v.fields.iter().any(|f| contains_rref(tcx, tcx.type_of(f.did))))
}
