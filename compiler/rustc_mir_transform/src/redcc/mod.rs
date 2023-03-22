mod embedck;
mod ptree;

use crate::MirPass;
use rustc_middle::mir::{visit::MutVisitor, Body, LocalDecls, Location, Place, Rvalue};
use rustc_middle::ty::{print::with_no_trimmed_paths, TyCtxt};

pub struct RRefEmbedCorrectionTransform;

// a rough plan for doing the rewriting:
// figure out how function calls are represented in mir
// modify the rvalue rref detection to return a path to any rvalue it discovers
// IDEA: turn this rref path detection into an iterator, rather than allocating space for it
// that is, assuming this is both possible and ergonomic
// figure out how to insert a function call in mir
// insert a call to some kind of "canary" function that just prints a debug message
// implement and insert a call to a function that basically does what we want
// IDEA: embed the clear call post-assignment. basically, do the assignment, _use_ the assignment place to know
// where to start your traversal, and then somehow scan the rvalue type for where the RRef(s) are. you'll then need
// path detection figure out how to get to those RRef(s), but for each one you can then insert a call to the clear function.

// take special care for arrays and tuples and other situations where there may be multiple RRefs in a single type.

impl<'tcx> MirPass<'tcx> for RRefEmbedCorrectionTransform {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        // FIXME: figure this clone thing out
        let mut visitor =
            RRefEmbedCorrectionTransformVisitor { tcx, local_decls: body.local_decls.clone() };
        visitor.visit_body(body);
    }
}

struct RRefEmbedCorrectionTransformVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    local_decls: LocalDecls<'tcx>,
}

impl<'tcx> MutVisitor<'tcx> for RRefEmbedCorrectionTransformVisitor<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_assign(&mut self, place: &mut Place<'tcx>, rvalue: &mut Rvalue<'tcx>, _: Location) {
        let place_ty = place.ty(&self.local_decls, self.tcx).ty;
        let rvalue_ty = rvalue.ty(&self.local_decls, self.tcx);

        // if embedck::place_contains_embedded_rref(*place, self.tcx, &self.local_decls)
        //     && embedck::ty_contains_rref(self.tcx, rvalue_ty)
        // {
        //     with_no_trimmed_paths!({
        //         println!("EMBED {} -> {}", rvalue_ty, place_ty);
        //     });
        // }

        if embedck::place_contains_embedded_rref(*place, self.tcx, &self.local_decls) {
            if let Some(node) = embedck::locate_embedded_rrefs(self.tcx, rvalue_ty) {
                node.traverse(*place, self.tcx, |place| {
                    let rref_place_ty = place.ty(&self.local_decls, self.tcx).ty;

                    with_no_trimmed_paths!({
                        println!("EMBED {} in {} -> {}", rref_place_ty, rvalue_ty, place_ty);
                    })
                });
            }
        }
    }
}
