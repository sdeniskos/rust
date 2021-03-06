import common::*;
import lib::llvm::{TypeRef};
import syntax::ast;
import lib::llvm::llvm;
import driver::session::session;
import std::map::hashmap;

import ty::*;

fn type_of_explicit_args(cx: @crate_ctxt, inputs: [ty::arg]) -> [TypeRef] {
    vec::map(inputs) {|arg|
        let arg_ty = arg.ty;
        let llty = type_of(cx, arg_ty);
        alt ty::resolved_mode(cx.tcx, arg.mode) {
          ast::by_val { llty }
          _ { T_ptr(llty) }
        }
    }
}

fn type_of_fn(cx: @crate_ctxt, inputs: [ty::arg], output: ty::t) -> TypeRef {
    let mut atys: [TypeRef] = [];

    // Arg 0: Output pointer.
    atys += [T_ptr(type_of(cx, output))];

    // Arg 1: Environment
    atys += [T_opaque_box_ptr(cx)];

    // ... then explicit args.
    atys += type_of_explicit_args(cx, inputs);
    ret T_fn(atys, llvm::LLVMVoidType());
}

// Given a function type and a count of ty params, construct an llvm type
fn type_of_fn_from_ty(cx: @crate_ctxt, fty: ty::t) -> TypeRef {
    type_of_fn(cx, ty::ty_fn_args(fty), ty::ty_fn_ret(fty))
}

fn type_of(cx: @crate_ctxt, t: ty::t) -> TypeRef {
    assert !ty::type_has_vars(t);
    // Check the cache.

    if cx.lltypes.contains_key(t) { ret cx.lltypes.get(t); }
    let llty = alt ty::get(t).struct {
      ty::ty_nil | ty::ty_bot { T_nil() }
      ty::ty_bool { T_bool() }
      ty::ty_int(t) { T_int_ty(cx, t) }
      ty::ty_uint(t) { T_uint_ty(cx, t) }
      ty::ty_float(t) { T_float_ty(cx, t) }
      ty::ty_estr(ty::vstore_uniq) |
      ty::ty_str { T_ptr(T_vec(cx, T_i8())) }
      ty::ty_enum(did, _) { type_of_enum(cx, did, t) }
      ty::ty_estr(ty::vstore_box) { T_ptr(T_box(cx, T_i8())) }
      ty::ty_evec(mt, ty::vstore_box) |
      ty::ty_box(mt) { T_ptr(T_box(cx, type_of(cx, mt.ty))) }
      ty::ty_opaque_box { T_ptr(T_box(cx, T_i8())) }
      ty::ty_uniq(mt) { T_ptr(type_of(cx, mt.ty)) }
      ty::ty_evec(mt, ty::vstore_uniq) |
      ty::ty_vec(mt) { T_ptr(T_vec(cx, type_of(cx, mt.ty))) }
      ty::ty_ptr(mt) { T_ptr(type_of(cx, mt.ty)) }
      ty::ty_rptr(_, mt) { T_ptr(type_of(cx, mt.ty)) }

      ty::ty_evec(mt, ty::vstore_slice(_)) {
        T_struct([T_ptr(type_of(cx, mt.ty)),
                  T_uint_ty(cx, ast::ty_u)])
      }

      ty::ty_estr(ty::vstore_slice(_)) {
        T_struct([T_ptr(T_i8()),
                  T_uint_ty(cx, ast::ty_u)])
      }

      ty::ty_estr(ty::vstore_fixed(n)) {
        T_array(T_i8(), n + 1u /* +1 for trailing null */)
      }

      ty::ty_evec(mt, ty::vstore_fixed(n)) {
        T_array(type_of(cx, mt.ty), n)
      }

      ty::ty_rec(fields) {
        let mut tys: [TypeRef] = [];
        for vec::each(fields) {|f|
            let mt_ty = f.mt.ty;
            tys += [type_of(cx, mt_ty)];
        }
        T_struct(tys)
      }
      ty::ty_fn(_) { T_fn_pair(cx, type_of_fn_from_ty(cx, t)) }
      ty::ty_iface(_, _) { T_opaque_iface(cx) }
      ty::ty_res(_, sub, substs) {
        let sub1 = ty::subst(cx.tcx, substs, sub);
        ret T_struct([T_i8(), type_of(cx, sub1)]);
      }
      ty::ty_param(_, _) { T_typaram(cx.tn) }
      ty::ty_type { T_ptr(cx.tydesc_type) }
      ty::ty_tup(elts) {
        let mut tys = [];
        for vec::each(elts) {|elt|
            tys += [type_of(cx, elt)];
        }
        T_struct(tys)
      }
      ty::ty_opaque_closure_ptr(_) { T_opaque_box_ptr(cx) }
      ty::ty_constr(subt,_) { type_of(cx, subt) }
      ty::ty_class(did, ts) {
        // only instance vars are record fields at runtime
        let fields = lookup_class_fields(cx.tcx, did);
        let tys = vec::map(fields) {|f|
            let t = ty::lookup_field_type(cx.tcx, did, f.id, ts);
            type_of(cx, t)
        };
        T_struct(tys)
      }
      ty::ty_self(_) { cx.tcx.sess.unimpl("type_of: ty_self \
                         not implemented"); }
      ty::ty_var(_) { cx.tcx.sess.bug("type_of shouldn't see a ty_var"); }
    };
    cx.lltypes.insert(t, llty);
    ret llty;
}

fn type_of_enum(cx: @crate_ctxt, did: ast::def_id, t: ty::t)
    -> TypeRef {
    let degen = (*ty::enum_variants(cx.tcx, did)).len() == 1u;
    let size = shape::static_size_of_enum(cx, t);
    if !degen { T_enum(cx, size) }
    else if size == 0u { T_struct([T_enum_variant(cx)]) }
    else { T_array(T_i8(), size) }
}
