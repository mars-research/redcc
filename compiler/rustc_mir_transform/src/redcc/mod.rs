mod embedck;

use crate::MirPass;
use rustc_middle::mir::{visit::MutVisitor, Body, LocalDecls, Location, Place, Rvalue};
use rustc_middle::ty::{print::with_no_trimmed_paths, TyCtxt};
use rustc_span::symbol::sym;

pub struct RRefEmbedTransform;

impl<'tcx> MirPass<'tcx> for RRefEmbedTransform {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        eprintln!("starting RRefEmbedTransform pass");
        eprintln!("RRef DefId: {:?}", tcx.get_diagnostic_item(sym::RRef));

        // FIXME: figure this thing out
        let mut visitor = RRefEmbedTransformVisitor { tcx, local_decls: body.local_decls.clone() };
        visitor.visit_body(body);
    }
}

struct RRefEmbedTransformVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    local_decls: LocalDecls<'tcx>,
}

impl<'tcx> MutVisitor<'tcx> for RRefEmbedTransformVisitor<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_assign(
        &mut self,
        place: &mut Place<'tcx>,
        rvalue: &mut Rvalue<'tcx>,
        location: Location,
    ) {
        let place_ty = self.local_decls[place.local].ty;
        let rvalue_ty = rvalue.ty(&self.local_decls, self.tcx);
        with_no_trimmed_paths!({
            eprintln!(
                "found an assignment of type {} to {}: {:?} = {:?} [{:?}]",
                place_ty, rvalue_ty, place, rvalue, location
            );
        });

        if embedck::place_contains_rref(*place, self.tcx, &self.local_decls) && embedck::contains_rref(self.tcx, rvalue_ty) {
            eprintln!("assignment embeds");
        }
    }
}
