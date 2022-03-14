use crate::MirPass;
use rustc_middle::mir::{Body, Location, Place, Rvalue};
use rustc_middle::mir::visit::MutVisitor;
use rustc_middle::ty::TyCtxt;

pub struct ShowAssign;

impl<'tcx> MirPass<'tcx> for ShowAssign {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        println!("starting ShowAssign pass");

        let mut visitor = ShowAssignVisitor(tcx);
        visitor.visit_body(body);
    }
}


struct ShowAssignVisitor<'tcx>(TyCtxt<'tcx>);

impl<'tcx> MutVisitor<'tcx> for ShowAssignVisitor<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.0
    }

    fn visit_assign(
        &mut self,
        _place: &mut Place<'tcx>,
        _rvalue: &mut Rvalue<'tcx>,
        _location: Location
    ) {
        println!("found an assignment");
    }
}
