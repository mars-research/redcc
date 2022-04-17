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
        let rval_ty = rvalue.ty(&self.local_decls, self.tcx);
        with_no_trimmed_paths!({
            eprintln!(
                "found an assignment of type {}: {:?} = {:?} [{:?}]",
                rval_ty, place, rvalue, location
            );
        });

        eprintln!("rvalue contains rref? {}", embedck::contains_rref(self.tcx, rval_ty));
    }
}
